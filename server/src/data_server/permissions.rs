use serde::{Deserialize, Serialize};
use structopt::clap::arg_enum;

arg_enum! {
    #[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
    pub enum InteractionPermission {
        Forbidden,
        YourSelf,
        SameClass,
        AnyBody,
    }
}

impl InteractionPermission {
    pub fn forbidden() -> Self {
        Self::Forbidden
    }

    pub fn is_forbidden(&self) -> bool {
        *self == Self::Forbidden
    }

    pub fn yourself() -> Self {
        Self::YourSelf
    }

    pub fn is_yourself(&self) -> bool {
        *self == Self::YourSelf
    }

    pub fn same_class() -> Self {
        Self::SameClass
    }

    pub fn is_same_class(&self) -> bool {
        *self == Self::SameClass
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Permissions {
    /// weather a user is allowed to vote/add nickname for someone
    #[serde(skip_serializing_if = "InteractionPermission::is_same_class")]
    #[serde(default = "InteractionPermission::same_class")]
    pub vote: InteractionPermission,
    /// weather a user is allowed to delete nickname for someone
    #[serde(skip_serializing_if = "InteractionPermission::is_yourself")]
    #[serde(default = "InteractionPermission::yourself")]
    pub delete: InteractionPermission,
    /// weather a user is allowed to protect someone nickname from deletion, only someone with the right to protect can delete these nicknames
    #[serde(skip_serializing_if = "InteractionPermission::is_forbidden")]
    #[serde(default = "InteractionPermission::forbidden")]
    pub protect_nickname: InteractionPermission,
    /// should we display cmd ?
    #[serde(skip_serializing_if = "not")]
    #[serde(default = "bool::default")]
    pub allowed_to_use_cmd: bool,
    /// can this user view others password ?
    #[serde(skip_serializing_if = "not")]
    #[serde(default = "bool::default")]
    pub allowed_to_change_passwords: bool,
    /// can this user view nickname datas ?
    #[serde(skip_serializing_if = "not")]
    #[serde(default = "bool::default")]
    pub allowed_to_view_nickname_data: bool,
    ///  other users permissions
    #[serde(skip_serializing_if = "not")]
    #[serde(default = "bool::default")]
    pub able_to_change_other_perm: bool,
}

fn not(b: &bool) -> bool {
    !b
}

impl Default for Permissions {
    fn default() -> Self {
        Self {
            vote: InteractionPermission::SameClass,
            delete: InteractionPermission::YourSelf,
            protect_nickname: InteractionPermission::Forbidden,
            allowed_to_use_cmd: false,
            allowed_to_change_passwords: false,
            allowed_to_view_nickname_data: false,
            able_to_change_other_perm: false,
        }
    }
}

impl Permissions {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}
