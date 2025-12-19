use std::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub struct ProfilID(pub u32);

impl Display for ProfilID {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

/// Used to log in
#[derive(Deserialize, Serialize, Debug, Clone, Default, Hash, Eq, PartialEq)]
pub struct Credentials {
    pub name: String,
    pub password: String,
}

/// A shortcut for a profil, this can be used publicly,
#[derive(Copy, Clone, Debug, Default, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub struct ClassID(pub u32);