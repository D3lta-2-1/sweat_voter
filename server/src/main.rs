mod commands;
mod data_server;

use crate::commands::{
    AddClass, AddLonelyToClass, AddProfil, AddToClass, ChangeName, ChangePassword,
    ChangePermission, DeleteClass, DeleteProfil, PermissionKind, RemoveFromClass, ViewPassword,
};
use crate::data_server::permissions::Permissions;
use crate::data_server::{DataServer, NickNameProposition, ServerError};
use actix_cors::Cors;
use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::http::KeepAlive;
use actix_web::{
    web, web::ServiceConfig, App, Either, HttpMessage, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use common::packets::c2s;
use common::packets::c2s::{
    AskForNicknameList, AskForProfilStats, CommandInput, DeleteNickname, Login,
    UpdateNicknameProtection, VoteNickname,
};
use common::packets::s2c::CommandResponse;
use common::ProfilID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::stdin;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Mutex;
use std::time::Duration;
use structopt::clap::AppSettings;
use structopt::StructOpt;
use tokio::task::spawn_blocking;
use tracing::info;

extern crate tracing;

type State = Mutex<AppState>;

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
enum SaveFormat {
    Cbor,
    Json,
}

struct AppState {
    data_server: DataServer,
    save_format: SaveFormat,
}

impl AppState {
    fn save(&mut self) {
        match self.save_format {
            SaveFormat::Json => {
                if let Some(nicknames) = self.data_server.try_to_save_nickname() {
                    let file = File::create("nicknames.json").unwrap();
                    serde_json::to_writer_pretty(file, &nicknames).unwrap()
                }

                if let Some((repartition, id_map)) = self.data_server.try_to_save_profils() {
                    let file = File::create("classes.json").unwrap();
                    serde_json::to_writer_pretty(file, &repartition).unwrap();
                    let file = File::create("id_map.json").unwrap();
                    serde_json::to_writer_pretty(file, &id_map).unwrap();
                }
            }

            SaveFormat::Cbor => {
                if let Some(nicknames) = self.data_server.try_to_save_nickname() {
                    let file = File::create("nicknames.cbor").unwrap();
                    ciborium::into_writer(&nicknames, file).unwrap()
                }

                if let Some((repartition, id_map)) = self.data_server.try_to_save_profils() {
                    let file = File::create("classes.cbor").unwrap();
                    ciborium::into_writer(&repartition, file).unwrap();
                    let file = File::create("id_map.cbor").unwrap();
                    ciborium::into_writer(&id_map, file).unwrap();
                }
            }
        }
    }

    /// return which file is the more recent, if unable to compare, return None,
    fn is_more_recent_than(f1: &File, f2: &File) -> Option<bool> {
        let time1 = f1.metadata().ok()?.modified().ok()?;
        let time2 = f2.metadata().ok()?.modified().ok()?;
        Some(time1 > time2)
    }

    /// load data from a file, automatically choose between cbor and json depending on which one is the latest
    fn load_data<T: for<'a> Deserialize<'a>>(format: SaveFormat, name: &str) -> Option<T> {
        let cbor = File::open(format!("{name}.cbor")).ok();
        let json = File::open(format!("{name}.json")).ok();

        match (cbor, json) {
            (Some(cbor), None) => {
                info!("loading {name}.cbor");
                ciborium::from_reader(cbor).ok()
            }
            (None, Some(json)) => {
                info!("loading {name}.json");
                serde_json::from_reader(json).ok()
            }
            (Some(cbor), Some(json)) => {
                if Self::is_more_recent_than(&cbor, &json).unwrap_or(format == SaveFormat::Cbor) {
                    info!("loading {name}.cbor");
                    ciborium::from_reader(cbor).ok()
                } else {
                    info!("loading {name}.json");
                    serde_json::from_reader(json).ok()
                }
            }
            (None, None) => None,
        }
    }

    fn new(save_format: SaveFormat) -> Mutex<Self> {
        let people_repartition =
            Self::load_data(save_format, "classes").unwrap_or(Default::default());
        let id_map = Self::load_data(save_format, "id_map").unwrap_or(Default::default());
        let mut data_server = DataServer::new(people_repartition, id_map);

        if let Some(nicknames) =
            Self::load_data::<HashMap<ProfilID, Vec<NickNameProposition>>>(save_format, "nicknames")
        {
            info!("{} nicknames loaded", nicknames.len());
            data_server.load_proposition(nicknames);
        }

        if let Some(generated_id_map) = data_server.build_id_map() {
            let file = File::create("id_map.json").expect("Failed to create a id_map file");
            serde_json::to_writer_pretty(file, &generated_id_map).unwrap();
        }

        Mutex::new(AppState {
            data_server,
            save_format,
        })
    }

    fn execute_command(&mut self, command: Commands) -> Result<Option<String>, ServerError> {
        let server = &mut self.data_server;
        match command {
            Commands::Exit => Ok(Some("You can't shutdown the server from here".to_string())),
            Commands::AddProfil(AddProfil { name, password }) => {
                server.add_profile(name, password).map(|_| None)
            }
            Commands::DeleteProfil(DeleteProfil { name }) => {
                server.delete_profil(name).map(|_| None)
            }
            Commands::AddClass(AddClass { name }) => server.add_class(name).map(|_| None),
            Commands::DeleteClass(DeleteClass { name }) => server.delete_class(name).map(|_| None),
            Commands::ViewLonelyPeople => {
                use std::fmt::Write;

                let peoples = server.find_people_out_of_any_class();
                let mut output = String::new();
                if peoples.is_empty() {
                    writeln!(&mut output, "No people found!").unwrap();
                } else {
                }
                for people in peoples {
                    writeln!(&mut output, "{}", people).unwrap();
                }
                Ok(Some(output))
            }
            Commands::AddLonelyPeopleToClass(AddLonelyToClass { class }) => {
                let people = server.find_id_out_of_any_class();
                for id in people {
                    server.add_to_class(id, &class)?;
                }
                Ok(None)
            }
            Commands::ViewPassword(ViewPassword { name }) => {
                let id = server.get_profil_id(&name)?;
                let password = server.get_password(id)?;
                Ok(Some(format!("{}'s password is {}", name, password)))
            }
            Commands::ChangePassword(ChangePassword { name, new_password }) => {
                let id = server.get_profil_id(&name)?;
                server.change_password(id, new_password)?;
                Ok(None)
            }
            Commands::ChangeName(ChangeName { name, new_name }) => {
                server.change_name(name, new_name).map(|_| None)
            }
            Commands::AddToClass(AddToClass {
                profil_name,
                class_name,
            }) => {
                let id = server.get_profil_id(&profil_name)?;
                server.add_to_class(id, &class_name)?;
                Ok(None)
            }
            Commands::RemoveFromClass(RemoveFromClass {
                profil_name,
                class_name,
            }) => {
                let id = server.get_profil_id(&profil_name)?;
                server.remove_from_class(id, class_name)?;
                Ok(None)
            }
            Commands::ChangePerm(ChangePermission { name, kind }) => {
                let id = server.get_profil_id(&name)?;
                let perm = server.get_permissions_mut(id)?;
                match kind {
                    PermissionKind::Vote { permission } => perm.vote = permission,
                    PermissionKind::Delete { permission } => perm.delete = permission,
                    PermissionKind::Protect { permission } => perm.protect_nickname = permission,
                    PermissionKind::UseCmd { permission } => perm.allowed_to_use_cmd = permission,
                }
                Ok(None)
            }
        }
    }
}

fn get_id(data_server: &DataServer, user: Option<actix_identity::Identity>) -> Option<ProfilID> {
    let name = user?.id().ok()?;
    data_server.get_profil_id(&name).ok()
}

#[actix_web::post("/login")]
async fn login(
    login: web::Json<Login>,
    req: HttpRequest,
    state: web::Data<State>,
) -> impl Responder {
    let server = &state.lock().unwrap().data_server;
    let id = server.log(&login.identity);
    if id.is_some() {
        actix_identity::Identity::login(&req.extensions(), login.identity.name.clone()).unwrap();
    };
    web::Json(server.class_list(id))
}

#[actix_web::post("/change_password")]
async fn change_password(
    new_password: web::Json<c2s::ChangePassword>,
    state: web::Data<State>,
    user: Option<actix_identity::Identity>,
) -> impl Responder {
    let server = &mut state.lock().unwrap().data_server;
    let Some(id) = get_id(&server, user) else {
        return HttpResponse::Unauthorized();
    };
    if server
        .change_password(id, new_password.0.new_password)
        .is_ok()
    {
        HttpResponse::Ok()
    } else {
        HttpResponse::BadRequest()
    }
}

#[actix_web::post("/logout")]
async fn logout(state: web::Data<State>, user: Option<actix_identity::Identity>) -> impl Responder {
    if let Some(user) = user {
        user.logout();
    }
    let server = &state.lock().unwrap().data_server;
    web::Json(server.class_list(None))
}

#[actix_web::get("/class_list")]
async fn list_class(
    state: web::Data<State>,
    user: Option<actix_identity::Identity>,
) -> impl Responder {
    let server = &state.lock().unwrap().data_server;
    let id = get_id(&server, user);
    web::Json(server.class_list(id))
}

#[actix_web::post("/nickname_list")]
async fn nickname_list(
    asked: web::Json<AskForNicknameList>,
    state: web::Data<State>,
    user: Option<actix_identity::Identity>,
) -> impl Responder {
    let AskForNicknameList { profil } = asked.0;
    let server = &state.lock().unwrap().data_server;
    let id = get_id(&server, user);
    web::Json(server.nickname_list(id, profil))
}

#[actix_web::post("/profil_stats")]
async fn profil_stats(
    asked: web::Json<AskForProfilStats>,
    state: web::Data<State>,
) -> impl Responder {
    let AskForProfilStats { profil } = asked.0;
    let server = &state.lock().unwrap().data_server;
    match server.profil_stats(profil) {
        None => Either::Left(HttpResponse::BadRequest()),
        Some(s) => Either::Right(web::Json(s)),
    }
}

#[actix_web::post("/vote_nickname")]
async fn vote_nickname(
    vote_nickname: web::Json<VoteNickname>,
    state: web::Data<State>,
    user: Option<actix_identity::Identity>,
) -> impl Responder {
    let VoteNickname { target, nickname } = vote_nickname.0;
    let server = &mut state.lock().unwrap().data_server;
    let id = get_id(&server, user);
    if let Some(id) = id {
        server.vote(id, target, nickname);
        Either::Left(web::Json(server.nickname_list(Some(id), target)))
    } else {
        Either::Right(HttpResponse::Unauthorized())
    }
}

#[actix_web::post("/delete_nickname")]
async fn delete_nickname(
    delete_nickname: web::Json<DeleteNickname>,
    state: web::Data<State>,
    user: Option<actix_identity::Identity>,
) -> impl Responder {
    let DeleteNickname { target, nickname } = delete_nickname.0;
    let server = &mut state.lock().unwrap().data_server;
    let id = get_id(&server, user);

    if let Some(id) = id {
        server.delete(id, target, nickname);
        Either::Left(web::Json(server.nickname_list(Some(id), target)))
    } else {
        Either::Right(HttpResponse::Unauthorized())
    }
}

#[actix_web::post("/update_nickname_protection")]
async fn update_protection_nickname(
    nickname_protection_update: web::Json<UpdateNicknameProtection>,
    state: web::Data<State>,
    user: Option<actix_identity::Identity>,
) -> impl Responder {
    let UpdateNicknameProtection {
        target,
        nickname,
        protection_statut,
    } = nickname_protection_update.0;
    let server = &mut state.lock().unwrap().data_server;
    let id = get_id(&server, user);

    if let Some(id) = id {
        server.update_nickname_protection(id, target, nickname, protection_statut);
        Either::Left(web::Json(server.nickname_list(Some(id), target)))
    } else {
        Either::Right(HttpResponse::Unauthorized())
    }
}

#[actix_web::post("/cmd_input")]
async fn cmd_input(
    cmd: web::Json<CommandInput>,
    state: web::Data<State>,
    user: Option<actix_identity::Identity>,
) -> impl Responder {
    let app = &mut state.lock().unwrap();
    let Some(id) = get_id(&app.data_server, user) else {
        return Either::Right(HttpResponse::Unauthorized());
    };

    if let Some(Permissions {
        allowed_to_use_cmd: false,
        ..
    }) = app.data_server.get_permission(id)
    {
        return Either::Right(HttpResponse::Unauthorized());
    };

    let Some(inputs) = shlex::split(&cmd.text) else {
        return Either::Left(web::Json(CommandResponse {
            text: "this command could not be parsed, check your quotes".to_string(),
        }));
    };

    let clap = Commands::clap().setting(AppSettings::NoBinaryName);
    let command = clap.get_matches_from_safe(inputs.iter().map(|input| input.trim()));
    let command = match command {
        Ok(command) => Commands::from_clap(&command),
        Err(e) => {
            return Either::Left(web::Json(CommandResponse {
                text: e.to_string(),
            }));
        }
    };

    let result = app.execute_command(command);
    let text = match result {
        Ok(None) => "action performed successfully!".to_string(),
        Ok(Some(result)) => result.trim().to_string(),
        Err(e) => e.to_string(),
    };

    Either::Left(web::Json(CommandResponse { text }))
}

async fn save_loop(state: web::Data<Mutex<AppState>>, duration: Duration) {
    let mut interval = actix_web::rt::time::interval(duration);
    loop {
        interval.tick().await;
        let mut state = state.lock().unwrap();
        state.save()
    }
}

#[derive(StructOpt)]
enum Commands {
    Exit,
    AddProfil(AddProfil),
    DeleteProfil(DeleteProfil),
    AddClass(AddClass),
    DeleteClass(DeleteClass),
    ViewLonelyPeople,
    AddLonelyPeopleToClass(AddLonelyToClass),
    ViewPassword(ViewPassword),
    ChangePassword(ChangePassword),
    ChangeName(ChangeName),
    AddToClass(AddToClass),
    RemoveFromClass(RemoveFromClass),
    ChangePerm(ChangePermission),
}

fn wait_for_cmd_input(server: web::Data<Mutex<AppState>>) {
    let mut command = String::new();
    loop {
        // read stdin
        command.clear();
        if let Err(e) = stdin().read_line(&mut command) {
            println!("{}", e);
            continue;
        }

        // parse the command
        let Some(inputs) = shlex::split(&command) else {
            println!("this command could not be parsed, check your quotes");
            continue;
        };

        let clap = Commands::clap().setting(AppSettings::NoBinaryName);
        let command = clap.get_matches_from_safe(inputs.iter().map(|input| input.trim()));
        let command = match command {
            Ok(command) => Commands::from_clap(&command),
            Err(e) => {
                println!("{}", e);
                continue;
            }
        };

        if let Commands::Exit = command {
            return;
        }

        let result = server.lock().unwrap().execute_command(command);
        match result {
            Ok(None) => println!("action performed successfully!"),
            Ok(Some(result)) => println!("{}", result.trim()),
            Err(e) => println!("{}", e),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ServerConfig {
    address: SocketAddr,
    save_intervals: Duration,
    save_format: SaveFormat,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 3000),
            save_intervals: Duration::from_secs(300),
            save_format: SaveFormat::Cbor,
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // install global subscriber configured based on RUST_LOG envvar.
    tracing_subscriber::fmt().init();
    let secret_key = Key::generate();

    let Ok(file) = File::open("config.json") else {
        let config = File::create("config.json").expect("failed to create config");
        serde_json::to_writer_pretty(config, &ServerConfig::default())?;
        info!("Config created");
        return Ok(());
    };
    let config: ServerConfig = serde_json::from_reader(file)?;

    info!("Starting server");

    let state = web::Data::new(AppState::new(config.save_format));

    let cloned = state.clone();
    let cloned2 = state.clone();
    tokio::spawn(save_loop(state.clone(), config.save_intervals));

    let signal = async || {
        spawn_blocking(move || wait_for_cmd_input(cloned))
            .await
            .unwrap();
    };

    let e = HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .app_data(web::Data::clone(&state))
            .wrap(IdentityMiddleware::default())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            //.wrap(Logger::default())
            .wrap(cors)
            .configure(routes)
            .service(Files::new("assets", "client/dist/assets").show_files_listing())
            .service(Files::new("", "client/dist/").index_file("index.html"))
    })
    .shutdown_signal(signal())
    .keep_alive(KeepAlive::Os)
    .bind(config.address)?
    .run()
    .await;

    info!("server stopping");
    cloned2.lock().unwrap().save();
    info!("content saved");
    e
}

fn routes(cfg: &mut ServiceConfig) {
    cfg.service(login);
    cfg.service(logout);
    cfg.service(change_password);
    cfg.service(list_class);
    cfg.service(nickname_list);
    cfg.service(profil_stats);
    cfg.service(delete_nickname);
    cfg.service(vote_nickname);
    cfg.service(update_protection_nickname);
    cfg.service(cmd_input);
}
