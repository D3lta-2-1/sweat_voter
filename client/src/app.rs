use crate::class_selector::ClassSelector;
use crate::console::{ConsoleBuilder, ConsoleEvent, ConsoleWindow};
use crate::login_selector::{EditorSelector, LoginAction};
use crate::nickname_viewer::{NickNameViewer, NicknameViewerAction};
use crate::person_selector::{PersonSelector, Selection};
use crate::stats_viewer::StatsViewer;
use common::packets::c2s::{
    AskForNicknameList, AskForProfilStats, ChangePassword, CommandInput, DeleteNickname, Login,
    UpdateNicknameProtection, VoteNickname,
};
use common::packets::s2c::{CommandResponse, LoginResponse, S2cPacket, S2cPackets};
use common::Identity;
use eframe::App;
use egui::{InnerResponse, TextBuffer};
use log::warn;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

pub struct HttpApp {
    incoming_message: Receiver<S2cPackets>,
    sender: Sender<S2cPackets>,
    editor_selector: EditorSelector,
    class_selector: ClassSelector,
    person_selector: PersonSelector,
    nickname_viewer: NickNameViewer,
    stats_viewer: StatsViewer,
    console: Option<ConsoleWindow>,
    ctx: egui::Context,
}

impl HttpApp {
    #[cfg(not(target_arch = "wasm32"))]
    const ROOT: &'static str = "https://sweat.corneille-rouen.xyz/";
    #[cfg(target_arch = "wasm32")]
    const ROOT: &'static str = "";
    const UNAUTHORIZED: u16 = 401;

    fn fetch(&self, request: ehttp::Request) {
        let new_sender = self.sender.clone();
        let ctx = self.ctx.clone();

        ehttp::fetch(request, move |response| {
            let response = match response {
                Ok(response) => response,
                Err(e) => {
                    warn!("Failed to fetch: {}", e);
                    return;
                }
            };

            // in the case of an unauthorized action, we clear everything, and we wait for the user to log...
            if response.status == Self::UNAUTHORIZED {
                // TODO: HANDLE THIS CASE
                /*let _ = new_sender
                    .send(S2cPackets::ClassList(LoginResponse::default()))
                    .expect("Failed to channel packet");
                ctx.request_repaint(); */
                return;
            }

            if let Some(packet) = response.json().ok() {
                let _ = new_sender.send(packet).expect("Failed to channel packet");
                ctx.request_repaint();
            }
        });
    }

    fn request_classes(&mut self) {
        let request = ehttp::Request::get(format!("{}class_list", Self::ROOT));
        self.fetch(request);
    }

    fn login(&mut self, identity: Identity) {
        let request = ehttp::Request::json(format!("{}login", Self::ROOT), &Login { identity })
            .expect("Failed to create request");
        self.fetch(request);
    }

    fn logout(&mut self) {
        let request = ehttp::Request::post(format!("{}logout", Self::ROOT), vec![]);
        self.fetch(request)
    }

    fn change_password(&mut self, new_password: String) {
        let request = ehttp::Request::json(
            format!("{}change_password", Self::ROOT),
            &ChangePassword { new_password },
        )
        .expect("failed_to_create_request");
        self.fetch(request)
    }

    fn input_cmd(&mut self, input: CommandInput) {
        let request = ehttp::Request::json(format!("{}cmd_input", Self::ROOT), &input)
            .expect("failed_to_create_request");
        self.fetch(request);
    }

    fn request_nickname_list(&mut self, ask_for_person_profil: AskForNicknameList) {
        let request = ehttp::Request::json(
            format!("{}nickname_list", Self::ROOT),
            &ask_for_person_profil,
        )
        .expect("Failed to create request");
        self.fetch(request);
    }

    fn request_profil_stats(&mut self, ask_for_person_profil: AskForProfilStats) {
        let request = ehttp::Request::json(
            format!("{}profil_stats", Self::ROOT),
            &ask_for_person_profil,
        )
        .expect("Failed to create request");
        self.fetch(request);
    }

    fn vote_nickname(&mut self, vote_nickname: VoteNickname) {
        let request = ehttp::Request::json(format!("{}vote_nickname", Self::ROOT), &vote_nickname)
            .expect("Failed to create request");
        self.fetch(request);
    }

    fn delete_nickname(&mut self, delete_nickname: DeleteNickname) {
        let request =
            ehttp::Request::json(format!("{}delete_nickname", Self::ROOT), &delete_nickname)
                .expect("Failed to create request");
        self.fetch(request);
    }

    fn update_nickname_protection(&mut self, update_nickname_protection: UpdateNicknameProtection) {
        let request = ehttp::Request::json(
            format!("{}update_nickname_protection", Self::ROOT),
            &update_nickname_protection,
        )
        .expect("Failed to create request");
        self.fetch(request);
    }

    fn check_incoming(&mut self) {
        let mut should_update_viewed_profil = false;

        for message in self
            .incoming_message
            .try_iter()
            .flat_map(|packets| packets.0.into_iter())
        {
            match message {
                S2cPacket::LoginResponse(class_list) => {
                    let LoginResponse {
                        logged,
                        allowed_to_use_cmd,
                    } = class_list;
                    should_update_viewed_profil = true;
                    self.editor_selector.set_logged(logged);
                    self.console = if allowed_to_use_cmd {
                        Some(
                            ConsoleBuilder::new()
                                .prompt(">> ")
                                .tab_quote_character('"')
                                .scrollback_size(100)
                                .history_size(100)
                                .build(),
                        )
                    } else {
                        None
                    };
                }
                S2cPacket::Classes(mut classes) => {
                    self.class_selector.set_classes(
                        classes
                            .classes
                            .iter_mut()
                            .map(|(id, class)| (*id, class.name.take()))
                            .collect(),
                    );
                    self.person_selector.set_classes(
                        classes
                            .classes
                            .into_iter()
                            .map(|(class_id, class)| (class_id, class.profiles)),
                    );
                }
                S2cPacket::NicknameList(person_profil_response) => {
                    self.nickname_viewer.set_profil(person_profil_response)
                }
                S2cPacket::ProfilStats(stats) => self.stats_viewer.set_stats(stats),
                S2cPacket::CommandResponse(CommandResponse { text }) => {
                    if let Some(console) = &mut self.console {
                        console.write(&text);
                        console.prompt();
                    }
                }
            }
        }
        if let Some(profil) = self
            .person_selector
            .get_selected_profil()
            .filter(|_| should_update_viewed_profil)
        {
            self.request_nickname_list(AskForNicknameList { profil })
        }
    }

    pub fn new(ctx: &eframe::CreationContext) -> Self {
        let editor_selector = EditorSelector::new(ctx.storage);
        let ctx = ctx.egui_ctx.clone();

        let (sender, incoming_message) = mpsc::channel();
        let mut this = Self {
            incoming_message,
            sender,
            editor_selector,
            class_selector: Default::default(),
            person_selector: Default::default(),
            nickname_viewer: Default::default(),
            stats_viewer: Default::default(),
            console: None,
            ctx,
        };
        this.request_classes();
        this.login(this.editor_selector.get_identity());
        this
    }
}

impl App for HttpApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.check_incoming();

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            let action = self.editor_selector.update(ui);

            match action {
                LoginAction::Login => {
                    self.login(self.editor_selector.get_identity());
                    if let Some(storage) = frame.storage_mut() {
                        self.editor_selector.save(storage)
                    }
                }
                LoginAction::Logout => self.logout(),
                LoginAction::ChangePassword(password) => {
                    if let Some(storage) = frame.storage_mut() {
                        self.editor_selector.save(storage)
                    }
                    self.change_password(password)
                }
                _ => (),
            }

            self.class_selector.update(ui);
        });

        if let Some(console) = &mut self.console {
            let inner = egui::Window::new("CMD")
                .default_open(false)
                .default_height(600.0)
                .resizable(true)
                .show(ctx, |ui| {
                    let result = console.draw(ui);
                    if let ConsoleEvent::Command(text) = result {
                        Some(text)
                    } else {
                        None
                    }
                });

            if let Some(InnerResponse {
                inner: Some(Some(text)),
                ..
            }) = inner
            {
                self.input_cmd(CommandInput { text })
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let Some(selected_class) = self.class_selector.get_selected() else {
                return;
            };

            // when has chosen a profil to view, we need to fetch it from the server
            let requested_profiles = self.person_selector.update(ui, selected_class);
            if let Some(profil) = requested_profiles {
                match profil {
                    Selection::ViewNickname(profil) => {
                        self.request_nickname_list(AskForNicknameList { profil })
                    }
                    Selection::ViewData(profil) => {
                        self.request_profil_stats(AskForProfilStats { profil })
                    }
                }
            }

            if let Some(selection) = self.person_selector.get_selection() {
                match selection {
                    Selection::ViewNickname(profil) => {
                        match self.nickname_viewer.update(ui, profil) {
                            NicknameViewerAction::Delete(delete_nickname) => {
                                self.delete_nickname(delete_nickname)
                            }
                            NicknameViewerAction::Vote(vote_nickname) => {
                                self.vote_nickname(vote_nickname)
                            }
                            NicknameViewerAction::UpdateProtection(update) => {
                                self.update_nickname_protection(update)
                            }
                            _ => {}
                        }
                    }
                    Selection::ViewData(profil) => self.stats_viewer.update(ui, profil),
                }
            }
        });
    }
}
