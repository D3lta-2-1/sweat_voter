use crate::password_selector::{PasswordSelector, Response};
use common::Identity;

/// Login field used on top of the UI
pub struct EditorSelector {
    name: String,
    password: String,
    logged: bool,
    change_password: Option<PasswordSelector>,
}

pub enum LoginAction {
    Login,
    Logout,
    ChangePassword(String),
    None,
}

impl EditorSelector {
    const NAME_KEY: &'static str = "name";
    const PASSWORD_KEY: &'static str = "password";

    pub fn new(storage: Option<&dyn eframe::Storage>) -> Self {
        Self {
            name: storage
                .map(|s| s.get_string(Self::NAME_KEY))
                .flatten()
                .unwrap_or(String::new()),
            password: storage
                .map(|s| s.get_string(Self::PASSWORD_KEY))
                .flatten()
                .unwrap_or(String::new()),
            logged: false,
            change_password: None,
        }
    }

    pub fn set_logged(&mut self, logged: bool) {
        self.logged = logged;
    }

    fn display_logout(&mut self, ui: &mut egui::Ui) -> bool {
        ui.label(format!("Connecté.e en tant que {}", self.name));
        ui.horizontal(|ui| {
            let r = ui.button("Se déconnecter").clicked();
            if self.change_password.is_none() && ui.button("Modifier le mot de passe").clicked() {
                self.change_password = Some(PasswordSelector::default());
            }
            r
        })
        .inner
    }

    pub fn update(&mut self, ui: &mut egui::Ui) -> LoginAction {
        if self.logged {
            if self.display_logout(ui) {
                LoginAction::Logout
            } else {
                match self
                    .change_password
                    .as_mut()
                    .map(|a| a.display(ui))
                    .unwrap_or(Response::None)
                {
                    Response::None => LoginAction::None,
                    Response::Back => {
                        self.change_password = None;
                        LoginAction::None
                    }
                    Response::Changed(new_password) => {
                        self.change_password = None;
                        self.password = new_password.clone();
                        LoginAction::ChangePassword(new_password)
                    }
                }
            }
        } else {
            if self.display_login(ui) {
                LoginAction::Login
            } else {
                LoginAction::None
            }
        }
    }

    fn display_login(&mut self, ui: &mut egui::Ui) -> bool {
        ui.label("Login");
        let name_response = ui
            .add(
                egui::TextEdit::singleline(&mut self.name)
                    .hint_text("Identifiant")
                    .char_limit(30),
            )
            .lost_focus();
        let password_response = ui
            .add(
                egui::TextEdit::singleline(&mut self.password)
                    .hint_text("Mot de passe")
                    .char_limit(30),
            )
            .lost_focus();

        ui.button("Valider").clicked()
            || (name_response || password_response)
                && !self.name.is_empty()
                && !self.password.is_empty()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
    }

    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        storage.set_string(Self::NAME_KEY, self.name.clone());
        storage.set_string(Self::PASSWORD_KEY, self.password.clone());
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_empty() | self.password.is_empty()
    }

    pub fn get_identity(&self) -> Identity {
        Identity {
            name: self.name.clone(),
            password: self.password.clone(),
        }
    }
}
