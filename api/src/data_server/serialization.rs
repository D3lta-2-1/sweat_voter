use crate::data_server::permissions::Permissions;
use crate::common::{ClassID, Credentials, ProfilID};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Profil {
    #[serde(flatten)]
    pub identity: Credentials,
    #[serde(default)]
    #[serde(skip_serializing_if = "Permissions::is_default")]
    pub permissions: Permissions,
}

#[derive(Serialize, Deserialize, Default)]
pub struct PeopleRepartition {
    pub profiles: Vec<Profil>,
    pub classes: Vec<Class>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct IdMap {
    pub profil_mapping: Vec<(ProfilID, String)>, //profil name <-> Id
    pub class_mapping: Vec<(ClassID, String)>,   //class name <-> Id
}

#[derive(Serialize, Deserialize)]
pub struct Class {
    pub name: String,
    pub people: Vec<String>,
}
