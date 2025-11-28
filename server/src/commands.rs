use crate::data_server::permissions::InteractionPermission;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = "Add a profile")]
pub struct AddProfil {
    pub name: String,
    pub password: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Remove a profile")]
pub struct DeleteProfil {
    pub name: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Create a new class")]
pub struct AddClass {
    pub name: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Remove a class")]
pub struct DeleteClass {
    pub name: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "View someone's password")] // Fuckin plaintext, we have sécuritéé
pub struct ViewPassword {
    pub name: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Change someone's password")]
pub struct ChangePassword {
    pub name: String,
    pub new_password: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Change someone's name")]
pub struct ChangeName {
    pub name: String,
    pub new_name: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Add someone to a class")]
pub struct AddToClass {
    pub profil_name: String,
    pub class_name: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Add to a class every person without an assigned class")]
pub struct AddLonelyToClass {
    pub class: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Remove a person from a class")] // what was "add remove to class"? x)
pub struct RemoveFromClass {
    pub profil_name: String,
    pub class_name: String,
}

#[derive(Debug, StructOpt)]
pub enum PermissionKind {
    Vote {
        permission: InteractionPermission,
    },
    Delete {
        permission: InteractionPermission,
    },
    Protect {
        permission: InteractionPermission,
    },
    UseCmd {
        #[structopt(parse(try_from_str))]
        permission: bool,
    },
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Change someone's permissions")]
pub struct ChangePermission {
    pub name: String,
    #[structopt(subcommand)]
    pub kind: PermissionKind,
}
