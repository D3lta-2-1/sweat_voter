use common::packets::c2s::{DeleteNickname, UpdateNicknameProtection, VoteNickname};
use common::packets::s2c;
use common::packets::s2c::NicknameStatut;
use common::ProfilID;
use egui::RichText;
use egui::TextBuffer;
use std::collections::HashMap;

struct Profile {
    allowed_to_vote: bool,
    allowed_to_protect: bool,
    nicknames: Vec<NicknameStatut>,
}

pub enum NicknameViewerAction {
    Vote(VoteNickname),
    Delete(DeleteNickname),
    UpdateProtection(UpdateNicknameProtection),
    None,
}

#[derive(Default)]
pub struct NickNameViewer {
    /// contain the profil
    profiles: HashMap<ProfilID, Profile>,
    /// edition field for a nickname proposition
    new_nickname: String,
}

impl NickNameViewer {
    /// used to cache a profil received by the server
    pub fn set_profil(&mut self, profil: s2c::NicknameList) {
        let s2c::NicknameList {
            profil_id,
            mut nicknames,
            allowed_to_vote,
            allowed_to_protect,
        } = profil;

        //always sort by the most voted !
        nicknames.sort_by(|a, b| {
            usize::cmp(&b.count, &a.count).then(String::cmp(&a.proposition, &b.proposition))
        });

        self.profiles.insert(
            profil_id,
            Profile {
                allowed_to_vote,
                allowed_to_protect,
                nicknames,
            },
        );
    }

    pub fn update(&mut self, ui: &mut egui::Ui, id: ProfilID) -> NicknameViewerAction {
        let mut action = NicknameViewerAction::None;

        let Some(profil) = self.profiles.get(&id) else {
            return action;
        };

        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("nicknames").striped(true).show(ui, |ui| {
                ui.heading("Surnoms");
                ui.heading("Votes");
                ui.end_row();

                for NicknameStatut {
                    proposition,
                    count,
                    contain_you,
                    allowed_to_be_delete,
                    protected,
                } in profil.nicknames.iter()
                {
                    ui.label(proposition);

                    let color = if *contain_you {
                        egui::Color32::from_rgb(255, 100, 100)
                    } else {
                        egui::Color32::from_rgb(100, 100, 255)
                    };

                    ui.label(RichText::new(count.to_string()).color(color));

                    if profil.allowed_to_vote && ui.button("Voter").clicked() {
                        //lazy evaluation hide the button if your not in the list
                        action = NicknameViewerAction::Vote(VoteNickname {
                            nickname: proposition.clone(),
                            target: id,
                        });
                    }

                    if *allowed_to_be_delete && ui.button("Supprimer").clicked() {
                        action = NicknameViewerAction::Delete(DeleteNickname {
                            nickname: proposition.clone(),
                            target: id,
                        });
                    }

                    if profil.allowed_to_protect {
                        let result = if *protected {
                            ui.button("Déverrouiller")
                        } else {
                            ui.button("Verrouiller")
                        };
                        if result.clicked() {
                            action =
                                NicknameViewerAction::UpdateProtection(UpdateNicknameProtection {
                                    target: id,
                                    nickname: proposition.clone(),
                                    protection_statut: !protected,
                                })
                        }
                    }

                    ui.end_row();
                }
            });
        });

        if profil.allowed_to_vote {
            ui.horizontal(|ui| {
                let submitted = ui.button("Proposer").clicked();
                let pressed_enter = ui
                    .add(
                        egui::TextEdit::singleline(&mut self.new_nickname)
                            .hint_text("Nouveau surnom")
                            .char_limit(30),
                    )
                    .lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if submitted || pressed_enter {
                    action = NicknameViewerAction::Vote(VoteNickname {
                        nickname: self.new_nickname.take(),
                        target: id,
                    });
                }
            });
        }
        action
    }
}
