use crate::data_server::permissions::Permissions;
use common::{ClassID, Identity, ProfilID};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Profil {
    #[serde(flatten)]
    pub identity: Identity,
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

#[derive(Serialize, Deserialize)]
pub struct ProfilSummary {
    pub profil_name: String,
    pub most_voted_nickname: String,
    pub votes_in_class: usize,
    pub total_votes: usize,
}