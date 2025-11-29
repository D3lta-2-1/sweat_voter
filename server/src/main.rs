mod app_state;
mod commands;
mod data_server;

use crate::app_state::{AppState, ChangedData, CommandOutput, SaveFormat};
use crate::commands::{
    AddClass, AddLonelyToClass, AddProfil, AddToClass, ChangeName, ChangePassword,
    ChangePermission, DeleteClass, DeleteProfil, RemoveFromClass, ViewPassword,
};
use crate::data_server::permissions::Permissions;
use crate::data_server::DataServer;
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
use common::packets::c2s::{
    AskForNicknameList, AskForProfilStats, CommandInput, DeleteNickname, Login,
    UpdateNicknameProtection, VoteNickname,
};
use common::packets::s2c::CommandResponse;
use common::packets::{c2s, s2c};
use common::ProfilID;
use serde::{Deserialize, Serialize};
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
    web::Json(s2c::S2cPackets::one(s2c::S2cPacket::LoginResponse(
        server.logged(id),
    )))
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
    web::Json(s2c::S2cPackets::one(s2c::S2cPacket::LoginResponse(
        server.logged(None),
    )))
}

#[actix_web::get("/class_list")]
async fn list_class(state: web::Data<State>) -> impl Responder {
    let server = &state.lock().unwrap().data_server;
    web::Json(s2c::S2cPackets::one(s2c::S2cPacket::Classes(
        server.class_list(),
    )))
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
    web::Json(s2c::S2cPackets::one(s2c::S2cPacket::NicknameList(
        server.nickname_list(id, profil),
    )))
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
        Some(s) => Either::Right(web::Json(s2c::S2cPackets::one(
            s2c::S2cPacket::ProfilStats(s),
        ))),
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
        Either::Left(web::Json(s2c::S2cPackets::one(
            s2c::S2cPacket::NicknameList(server.nickname_list(Some(id), target)),
        )))
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
        Either::Left(web::Json(s2c::S2cPackets::one(
            s2c::S2cPacket::NicknameList(server.nickname_list(Some(id), target)),
        )))
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
        Either::Left(web::Json(s2c::S2cPackets::one(
            s2c::S2cPacket::NicknameList(server.nickname_list(Some(id), target)),
        )))
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
        return Either::Left(web::Json(s2c::S2cPackets::one(
            s2c::S2cPacket::CommandResponse(CommandResponse {
                text: "this command could not be parsed, check your quotes".to_string(),
            }),
        )));
    };

    let clap = Commands::clap()
        .setting(AppSettings::NoBinaryName)
        .setting(AppSettings::ColorNever);
    let command = clap.get_matches_from_safe(inputs.iter().map(|input| input.trim()));
    let command = match command {
        Ok(command) => Commands::from_clap(&command),
        Err(e) => {
            return Either::Left(web::Json(s2c::S2cPackets::one(
                s2c::S2cPacket::CommandResponse(CommandResponse {
                    text: e.to_string(),
                }),
            )));
        }
    };

    let result = app.execute_command(command);
    let (text, action) = match result {
        Ok(CommandOutput {
            message: None,
            changed_data,
        }) => ("action performed successfully!".to_string(), changed_data),
        Ok(CommandOutput {
            message: Some(result),
            changed_data,
        }) => (result.trim().to_string(), changed_data),
        Err(e) => (e.to_string(), None),
    };

    let packets = s2c::S2cPackets(match action {
        None => vec![s2c::S2cPacket::CommandResponse(CommandResponse { text })],
        Some(ChangedData::Classes) => vec![
            s2c::S2cPacket::CommandResponse(CommandResponse { text }),
            s2c::S2cPacket::Classes(app.data_server.class_list()),
        ],
    });

    Either::Left(web::Json(packets))
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
            Ok(CommandOutput { message: None, .. }) => println!("action performed successfully!"),
            Ok(CommandOutput {
                message: Some(result),
                ..
            }) => println!("{}", result.trim()),
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
