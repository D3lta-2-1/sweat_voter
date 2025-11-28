use common::packets::s2c::ProfilStats;
use common::ProfilID;
use std::collections::HashMap;

#[derive(Default)]
pub struct StatsViewer {
    /// contain the profil
    profiles: HashMap<ProfilID, ProfilStats>,
}

impl StatsViewer {
    pub fn set_stats(&mut self, stats: ProfilStats) {
        self.profiles.insert(stats.profil_id, stats);
    }

    pub fn update(&self, ui: &mut egui::Ui, id: ProfilID) {
        let Some(stats) = self.profiles.get(&id) else {
            return;
        };
        egui::Grid::new("nicknames").striped(true).show(ui, |ui| {
            ui.label("Classe(s)");
            ui.label(stats.numbers_of_classes.to_string());
            ui.end_row();

            ui.label("Votes donnés");
            ui.label(stats.total_votes.to_string());
            ui.end_row();

            ui.label("Surnoms proposés");
            ui.label(stats.total_propositions.to_string());
            ui.end_row();

            ui.label("Surnoms reçus");
            ui.label(stats.numbers_of_nickname.to_string());
            ui.end_row();
        });
    }
}
