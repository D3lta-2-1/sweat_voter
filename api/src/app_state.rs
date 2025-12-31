use crate::commands::*;
use crate::common::{Credentials, ProfilID};
use crate::data_server::{DataServer, NickNameProposition, ServerError};
use axum_login::{AuthUser, AuthnBackend, UserId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;
use tracing::info;

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SaveFormat {
    Cbor,
    Json,
}

pub struct AppStateInner {
    pub data_server: Mutex<DataServer>,
    pub save_format: SaveFormat,
}

#[derive(Clone)]
pub struct AppState(Arc<AppStateInner>);

impl AppState {
    pub fn new(save_format: SaveFormat) -> Self {
        Self(Arc::new(AppStateInner::new(save_format)))
    }
}

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Copy, Clone, Debug)]
pub struct User {
    pub id: ProfilID,
    pub password_hash: u64,
}

impl AuthUser for User {
    type Id = ProfilID;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        bytemuck::bytes_of(&self.password_hash)
    }
}

impl AuthnBackend for AppState {
    type User = User;
    type Credentials = Credentials;
    type Error = std::convert::Infallible;

    async fn authenticate(
        &self,
        cred: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let server = self.data_server.lock().expect("mutex poisoned");

        Ok(server.log(&cred).map(|id| {
            let mut hasher = DefaultHasher::new();
            cred.password.hash(&mut hasher);
            User {
                id,
                password_hash: hasher.finish(),
            }
        }))
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        let server = self.data_server.lock().expect("mutex poisoned");

        let profil = server.get_profil(*user_id);

        Ok(profil.map(|profil| {
            let mut hasher = DefaultHasher::new();
            profil.identity.password.hash(&mut hasher);
            User {
                id: *user_id,
                password_hash: hasher.finish(),
            }
        }))
    }
}

#[derive(StructOpt)]
pub enum Commands {
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
    ViewNicknameData(ViewNicknameData),
    AddClassToClass(AddClassToClass),
}

/// used to signal if a something needs to be resent to the client.
///
#[derive(Copy, Clone)]
pub enum ChangedData {
    Classes,
}

#[derive(Clone, Default)]
pub struct CommandOutput {
    pub message: Option<String>,
    pub changed_data: Option<ChangedData>,
}

impl CommandOutput {
    pub fn update_classes() -> Self {
        Self {
            message: None,
            changed_data: Some(ChangedData::Classes),
        }
    }

    pub fn with_text(text: String) -> Self {
        Self {
            message: Some(text),
            changed_data: None,
        }
    }
}

impl AppStateInner {
    pub fn save(&self) {
        let mut server = self.data_server.lock().unwrap();

        match self.save_format {
            SaveFormat::Json => {
                if let Some(nicknames) = server.try_to_save_nickname() {
                    let file = File::create("nicknames.json").unwrap();
                    serde_json::to_writer_pretty(file, &nicknames).unwrap()
                }

                if let Some((repartition, id_map)) = server.try_to_save_profils() {
                    let file = File::create("classes.json").unwrap();
                    serde_json::to_writer_pretty(file, &repartition).unwrap();
                    let file = File::create("id_map.json").unwrap();
                    serde_json::to_writer_pretty(file, &id_map).unwrap();
                }
            }

            SaveFormat::Cbor => {
                if let Some(nicknames) = server.try_to_save_nickname() {
                    let file = File::create("nicknames.cbor").unwrap();
                    ciborium::into_writer(&nicknames, file).unwrap()
                }

                if let Some((repartition, id_map)) = server.try_to_save_profils() {
                    let file = File::create("classes.cbor").unwrap();
                    ciborium::into_writer(&repartition, file).unwrap();
                    let file = File::create("id_map.cbor").unwrap();
                    ciborium::into_writer(&id_map, file).unwrap();
                }
            }
        }
    }

    /// return which file is the more recent, if unable to compare, return None,
    pub fn is_more_recent_than(f1: &File, f2: &File) -> Option<bool> {
        let time1 = f1.metadata().ok()?.modified().ok()?;
        let time2 = f2.metadata().ok()?.modified().ok()?;
        Some(time1 > time2)
    }

    /// load data from a file, automatically choose between cbor and json depending on which one is the latest
    pub fn load_data<T: for<'a> Deserialize<'a>>(format: SaveFormat, name: &str) -> Option<T> {
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

    pub fn new(save_format: SaveFormat) -> Self {
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

        AppStateInner {
            data_server: Mutex::new(data_server),
            save_format,
        }
    }

    pub fn execute_command(
        &self,
        admin: Option<ProfilID>,
        command: Commands,
    ) -> Result<CommandOutput, ServerError> {
        let mut server = self.data_server.lock().unwrap();
        Ok(match command {
            Commands::Exit => CommandOutput {
                message: Some("You can't shutdown the api from here".to_string()),
                changed_data: None,
            },
            Commands::AddProfil(AddProfil { name, password }) => {
                server.add_profile(name, password)?;
                CommandOutput::default()
            }
            Commands::DeleteProfil(DeleteProfil { name }) => {
                server.delete_profil(name)?;
                CommandOutput::update_classes()
            }
            Commands::AddClass(AddClass { name }) => {
                server.add_class(name)?;
                CommandOutput::update_classes()
            }
            Commands::DeleteClass(DeleteClass { name }) => {
                server.delete_class(name)?;
                CommandOutput::update_classes()
            }
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
                CommandOutput::with_text(output)
            }
            Commands::AddLonelyPeopleToClass(AddLonelyToClass { class }) => {
                let people = server.find_id_out_of_any_class();
                server.add_many_to_class(people.into_iter(), &class)?;
                CommandOutput::update_classes()
            }
            Commands::ViewPassword(ViewPassword { name }) => {
                let id = server.get_profil_id(&name)?;
                let password = server.get_password(admin, id)?;
                CommandOutput::with_text(format!("{}'s password is {}", name, password))
            }
            Commands::ChangePassword(ChangePassword { name, new_password }) => {
                let id = server.get_profil_id(&name)?;
                server.change_password(admin, id, new_password)?;
                CommandOutput::default()
            }
            Commands::ChangeName(ChangeName { name, new_name }) => {
                server.change_name(name, new_name)?;
                CommandOutput::update_classes()
            }
            Commands::AddToClass(AddToClass {
                profil_name,
                class_name,
            }) => {
                let id = server.get_profil_id(&profil_name)?;
                server.add_to_class(id, &class_name)?;
                CommandOutput::update_classes()
            }
            Commands::RemoveFromClass(RemoveFromClass {
                profil_name,
                class_name,
            }) => {
                let id = server.get_profil_id(&profil_name)?;
                server.remove_from_class(id, class_name)?;
                CommandOutput::update_classes()
            }
            Commands::ChangePerm(ChangePermission { name, kind }) => {
                let id = server.get_profil_id(&name)?;
                let perm = server.get_permissions_mut(admin, id)?;
                match kind {
                    PermissionKind::Vote { permission } => perm.vote = permission,
                    PermissionKind::Delete { permission } => perm.delete = permission,
                    PermissionKind::Protect { permission } => perm.protect_nickname = permission,
                    PermissionKind::UseCmd { permission } => perm.allowed_to_use_cmd = permission,
                    PermissionKind::ChangePasswords { permission } => {
                        perm.allowed_to_change_passwords = permission
                    }
                    PermissionKind::ViewNicknameData { permission } => {
                        perm.allowed_to_view_nickname_data = permission
                    }
                    PermissionKind::ChangeOtherPerm { permission } => {
                        perm.able_to_change_other_perm = permission
                    }
                }
                CommandOutput::default()
            }
            Commands::ViewNicknameData(ViewNicknameData { owner, nickname }) => {
                use std::fmt::Write;

                let proposition = server.get_nickname(admin, owner, nickname)?;

                let mut output = String::new();
                writeln!(
                    &mut output,
                    "{} as been proposed by {} and voted by:",
                    proposition.proposition,
                    server
                        .get_profil(proposition.author)
                        .map(|p| p.identity.name.as_str())
                        .unwrap_or("[unknown]")
                )
                .unwrap();
                for voters in &proposition.votes {
                    writeln!(
                        &mut output,
                        "\t{}",
                        server
                            .get_profil(*voters)
                            .map(|p| p.identity.name.as_str())
                            .unwrap_or("[unknown]")
                    )
                    .unwrap();
                }
                CommandOutput::with_text(output)
            }
            Commands::AddClassToClass(AddClassToClass { class, target }) => {
                let class: Vec<_> = server.get_class(class)?.profiles.iter().copied().collect();
                server.add_many_to_class(class.into_iter(), &target)?;
                CommandOutput::update_classes()
            }
        })
    }
}
