pub mod c2s {
    use crate::{Identity, ProfilID};
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct Login {
        pub identity: Identity,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct ChangePassword {
        pub new_password: String,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct CommandInput {
        pub text: String,
    }

    /// Voting and adding a nickname is the same operation
    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct VoteNickname {
        pub target: ProfilID,
        pub nickname: String,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct DeleteNickname {
        pub target: ProfilID,
        pub nickname: String,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct AskForNicknameList {
        pub profil: ProfilID,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct AskForProfilStats {
        pub profil: ProfilID,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct UpdateNicknameProtection {
        pub target: ProfilID,
        pub nickname: String,
        pub protection_statut: bool,
    }
}

pub mod s2c {
    use crate::{ClassID, ProfilID};
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct S2cPackets(pub Vec<S2cPacket>);

    impl S2cPackets {
        pub fn one(p: S2cPacket) -> Self {
            S2cPackets(vec![p])
        }
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub enum S2cPacket {
        CommandResponse(CommandResponse),
        LoginResponse(LoginResponse),
        NicknameList(NicknameList),
        ProfilStats(ProfilStats),
        Classes(Classes),
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct CommandResponse {
        pub text: String,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct Class {
        pub name: String,
        pub profiles: Vec<(ProfilID, String)>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone, Default)]
    pub struct LoginResponse {
        pub logged: bool,
        pub allowed_to_use_cmd: bool,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct Classes {
        pub classes: Vec<(ClassID, Class)>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct NicknameStatut {
        pub proposition: String,
        pub count: usize,
        pub contain_you: bool,
        pub allowed_to_be_delete: bool,
        pub protected: bool,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct NicknameList {
        pub profil_id: ProfilID,
        pub nicknames: Vec<NicknameStatut>,
        pub allowed_to_vote: bool,
        pub allowed_to_protect: bool,
    }

    #[derive(Deserialize, Serialize, Debug, Clone, Default)]
    pub struct ProfilStats {
        pub profil_id: ProfilID,
        pub total_votes: i32,
        pub total_propositions: i32,
        pub numbers_of_nickname: usize,
        pub numbers_of_classes: usize,
    }
}
