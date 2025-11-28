use common::ClassID;
use egui::Spinner;

/// List of classes used to display
#[derive(Default)]
pub struct ClassSelector {
    classes: Vec<(ClassID, String)>,
    selected: Option<ClassID>,
}

impl ClassSelector {
    pub fn set_classes(&mut self, classes: Vec<(ClassID, String)>) {
        self.classes = classes;
        self.classes.sort_by(|(_, a), (_, b)| a.cmp(b));
        self.selected = self.classes.first().map(|(id, _)| *id);
    }

    /// return true when the selection has changed
    pub fn update(&mut self, ui: &mut egui::Ui) -> bool {
        if self.classes.is_empty() {
            ui.add(Spinner::new());
            ui.label("Aucune classe disponible");
            return false;
        }

        let mut changed = false;
        ui.label("Choisir une classe");
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                for (id, name) in self.classes.iter() {
                    changed |= ui
                        .selectable_value(&mut self.selected, Some(*id), name)
                        .changed();
                }
            });
        });

        changed
    }

    pub fn get_selected(&self) -> Option<ClassID> {
        self.selected
    }
}
