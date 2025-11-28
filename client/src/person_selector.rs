use common::{ClassID, ProfilID};

use crate::person_selector::Selection::{ViewData, ViewNickname};
use crate::person_selector::ViewMode::{Data, Nickname};
use std::collections::HashMap;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Default)]
enum ViewMode {
    #[default]
    Nickname,
    Data,
}

pub enum Selection {
    ViewNickname(ProfilID),
    ViewData(ProfilID),
}

#[derive(Default)]
pub struct PersonSelector {
    /// who is in which class
    classes: HashMap<ClassID, Vec<(ProfilID, String)>>,
    /// current profil viewed
    selected_profil: Option<ProfilID>,
    view_mode: ViewMode,
}

impl PersonSelector {
    pub fn get_selection(&self) -> Option<Selection> {
        let id = self.selected_profil?;
        Some(match self.view_mode {
            Nickname => ViewNickname(id),
            Data => ViewData(id),
        })
    }

    pub fn get_selected_profil(&self) -> Option<ProfilID> {
        self.selected_profil
    }

    pub fn set_classes<T: Iterator<Item = (ClassID, Vec<(ProfilID, String)>)>>(&mut self, iter: T) {
        let iter = iter.map(|(id, mut profiles)| {
            profiles.sort_unstable_by(|(_, a), (_, b)| a.cmp(b));
            (id, profiles)
        });
        self.classes = HashMap::from_iter(iter)
    }

    /// Profil selector, take which class to display and return which profil is requested
    pub fn update(&mut self, ui: &mut egui::Ui, class_id: ClassID) -> Option<Selection> {
        let Some(profils) = self.classes.get(&class_id) else {
            return None;
        };

        let mut requested_profil = None;

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if ui
                        .horizontal(|ui| {
                            ui.selectable_value(&mut self.view_mode, Nickname, "Voter")
                                .clicked()
                                || ui
                                    .selectable_value(&mut self.view_mode, Data, "Statistiques")
                                    .clicked()
                        })
                        .inner
                    {
                        requested_profil = self.selected_profil;
                    }

                    ui.heading("Participants");
                    ui.label("Choix du participant");
                    for (id, name) in profils {
                        if ui
                            .selectable_value(&mut self.selected_profil, Some(*id), name.as_str())
                            .changed()
                        {
                            requested_profil = Some(*id);
                        }
                    }
                });
            });
        Some(match self.view_mode {
            Nickname => ViewNickname(requested_profil?),
            Data => ViewData(requested_profil?),
        })
    }
}
