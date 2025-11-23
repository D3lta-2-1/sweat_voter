use egui::{TextBuffer, TextEdit, Widget};

#[derive(Default)]
pub struct PasswordSelector {
    entry1: String,
    entry2: String,
}

pub enum Response {
    None,
    Back,
    Changed(String),
}

impl PasswordSelector {
    pub fn display(&mut self, ui: &mut egui::Ui) -> Response {
        ui.label("Modifier le mot de passe");
        TextEdit::singleline(&mut self.entry1)
            .hint_text("Nouveau mot de passe")
            .ui(ui);
        TextEdit::singleline(&mut self.entry2)
            .hint_text("Confirmer le nouveau mot de passe")
            .ui(ui);

        ui.horizontal(|ui| {
            if ui.button("Confirmer").clicked() && self.entry1 == self.entry2 {
                self.entry2.clear();
                return Response::Changed(self.entry1.take());
            };
            if ui.button("Annuler").clicked() {
                return Response::Back;
            }
            Response::None
        })
        .inner
    }
}
