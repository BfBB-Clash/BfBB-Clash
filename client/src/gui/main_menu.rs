use std::rc::Rc;

use clash::{net::Message, player::PlayerOptions};
use eframe::{
    egui::{Align, CentralPanel, Layout, TextEdit, TopBottomPanel},
    App,
};

use crate::gui::state::{Screen, State};
use crate::gui::BORDER;

enum Submenu {
    Root,
    Host,
    Join,
}

pub struct MainMenu {
    state: Rc<State>,
    network_sender: tokio::sync::mpsc::Sender<Message>,
    submenu: Submenu,

    name_buf: String,

    lobby_id_buf: String,
    lobby_id: Option<u32>,
}

impl MainMenu {
    pub fn new(state: Rc<State>, network_sender: tokio::sync::mpsc::Sender<Message>) -> Self {
        Self {
            state,
            network_sender,
            submenu: Submenu::Root,
            name_buf: Default::default(),
            lobby_id_buf: Default::default(),
            lobby_id: Default::default(),
        }
    }
}

impl App for MainMenu {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        match self.submenu {
            Submenu::Root => {
                CentralPanel::default().show(ctx, |ui| {
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        ui.image(&self.state.logo, self.state.logo.size_vec2() / 3.);
                    });
                    ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                        ui.add_space(BORDER);
                        if ui.button("Quit").clicked() {
                            frame.close();
                        }
                        if ui.button("Join Game").clicked() {
                            self.submenu = Submenu::Join;
                        }
                        if ui.button("Host Game").clicked() {
                            self.submenu = Submenu::Host;
                        }
                    });
                });
            }
            Submenu::Host => {
                TopBottomPanel::top("Title").show(ctx, |ui| {
                    ui.vertical_centered(|ui| ui.label("Host Game"));
                });
                TopBottomPanel::bottom("Join Panel").show(ctx, |ui| {
                    ui.add(TextEdit::singleline(&mut self.name_buf).hint_text("Name"));
                    ui.add_enabled_ui(!self.name_buf.is_empty(), |ui| {
                        if ui.button("Host Game").clicked() {
                            self.network_sender
                                .blocking_send(Message::GameHost)
                                .unwrap();
                            self.network_sender
                                .blocking_send(Message::PlayerOptions {
                                    options: PlayerOptions {
                                        name: self.name_buf.clone(),
                                        color: (0, 0, 0),
                                    },
                                })
                                .unwrap();

                            self.state.screen.set(Screen::Lobby);
                        }
                    });
                    if ui.button("Back").clicked() {
                        self.submenu = Submenu::Root;
                    }
                    ui.add_space(BORDER);
                });
            }
            Submenu::Join => {
                TopBottomPanel::top("Title").show(ctx, |ui| {
                    ui.label("Join Game");
                });
                TopBottomPanel::bottom("Host Panel").show(ctx, |ui| {
                    ui.add(TextEdit::singleline(&mut self.name_buf).hint_text("Name"));
                    let lobby_response = ui.add(
                        TextEdit::singleline(&mut self.lobby_id_buf)
                            .hint_text("Lobby ID")
                            .password(true),
                    );

                    // Validate input
                    if lobby_response.changed() {
                        self.lobby_id = u32::from_str_radix(self.lobby_id_buf.as_str(), 16).ok();
                    }

                    let mut join_response = ui
                        .add_enabled_ui(self.lobby_id.is_some(), |ui| ui.button("Join Game"))
                        .inner;

                    if self.lobby_id.is_none() {
                        join_response = join_response.on_disabled_hover_text(
                            "Lobby ID must be an 8 digit hexadecimal number",
                        )
                    };

                    if join_response.clicked() {
                        self.network_sender
                            .blocking_send(Message::GameJoin {
                                lobby_id: self.lobby_id.unwrap(),
                            })
                            .unwrap();
                        self.network_sender
                            .blocking_send(Message::PlayerOptions {
                                options: PlayerOptions {
                                    name: self.name_buf.clone(),
                                    color: (0, 0, 0),
                                },
                            })
                            .unwrap();
                        self.state.screen.set(Screen::Lobby);
                    }

                    if ui.button("Back").clicked() {
                        self.submenu = Submenu::Root;
                    }
                    ui.add_space(BORDER);
                });
            }
        }
    }
}
