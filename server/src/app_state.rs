use crate::commands::{
    AddClass, AddLonelyToClass, AddProfil, AddToClass, ChangeName, ChangePassword,
    ChangePermission, DeleteClass, DeleteProfil, PermissionKind, RemoveFromClass, ViewPassword,
};
use crate::data_server::{DataServer, NickNameProposition, ServerError};
use crate::Commands;
use common::ProfilID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::sync::Mutex;
use tracing::info;

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SaveFormat {
    Cbor,
    Json,
}

pub struct AppState {
    pub data_server: DataServer,
    pub save_format: SaveFormat,
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
}

impl AppState {
    pub fn save(&mut self) {
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

    pub fn new(save_format: SaveFormat) -> Mutex<Self> {
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

    pub fn execute_command(&mut self, command: Commands) -> Result<CommandOutput, ServerError> {
        let server = &mut self.data_server;
        Ok(match command {
            Commands::Exit => CommandOutput {
                message: Some("You can't shutdown the server from here".to_string()),
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
                CommandOutput {
                    message: Some(output),
                    changed_data: None,
                }
            }
            Commands::AddLonelyPeopleToClass(AddLonelyToClass { class }) => {
                let people = server.find_id_out_of_any_class();
                for id in people {
                    server.add_to_class(id, &class)?;
                }
                CommandOutput::update_classes()
            }
            Commands::ViewPassword(ViewPassword { name }) => {
                let id = server.get_profil_id(&name)?;
                let password = server.get_password(id)?;
                CommandOutput {
                    message: Some(format!("{}'s password is {}", name, password)),
                    changed_data: None,
                }
            }
            Commands::ChangePassword(ChangePassword { name, new_password }) => {
                let id = server.get_profil_id(&name)?;
                server.change_password(id, new_password)?;
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
                let perm = server.get_permissions_mut(id)?;
                match kind {
                    PermissionKind::Vote { permission } => perm.vote = permission,
                    PermissionKind::Delete { permission } => perm.delete = permission,
                    PermissionKind::Protect { permission } => perm.protect_nickname = permission,
                    PermissionKind::UseCmd { permission } => perm.allowed_to_use_cmd = permission,
                }
                CommandOutput::default()
            }
        })
    }
}
