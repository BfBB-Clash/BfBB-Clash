use std::rc::Rc;

use clash_lib::{net::Message, player::PlayerOptions};
use eframe::{
    egui::{Align, Button, CentralPanel, Layout, TextEdit, TopBottomPanel},
    App,
};

use crate::gui::state::{Screen, State, Submenu};
use crate::gui::BORDER;

use super::val_text::ValText;

pub struct MainMenu {
    state: Rc<State>,
    network_sender: tokio::sync::mpsc::Sender<Message>,
    player_name: String,
    lobby_id: ValText<u32>,
}

impl MainMenu {
    pub fn new(state: Rc<State>, network_sender: tokio::sync::mpsc::Sender<Message>) -> Self {
        Self {
            state,
            network_sender,
            player_name: Default::default(),
            // TODO: atm it is not strictly true that the lobby_id must be 8 digits,
            //  since it's just a random u32. When this is resolved on the server-side,
            //  we need to validate it here as well.
            lobby_id: ValText::with_validator(|text| u32::from_str_radix(text, 16).ok()),
        }
    }
}

impl App for MainMenu {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        let submenu = match self.state.screen.get() {
            Screen::MainMenu(submenu) => submenu,
            _ => unreachable!("Attempted to extract Main Menu submenu while not on Main Menu"),
        };
        match submenu {
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
                            self.state.screen.set(Screen::MainMenu(Submenu::Join));
                        }
                        if ui.button("Host Game").clicked() {
                            self.state.screen.set(Screen::MainMenu(Submenu::Host));
                        }
                    });
                });
            }
            Submenu::Host => {
                TopBottomPanel::top("Title").show(ctx, |ui| {
                    ui.vertical_centered(|ui| ui.label("Host Game"));
                });
                TopBottomPanel::bottom("Join Panel").show(ctx, |ui| {
                    ui.add(TextEdit::singleline(&mut self.player_name).hint_text("Name"));
                    ui.add_enabled_ui(!self.player_name.is_empty(), |ui| {
                        if ui.button("Host Game").clicked() {
                            self.network_sender
                                .blocking_send(Message::GameHost)
                                .unwrap();
                            self.network_sender
                                .blocking_send(Message::PlayerOptions {
                                    options: PlayerOptions {
                                        name: self.player_name.clone(),
                                        color: (0, 0, 0),
                                    },
                                })
                                .unwrap();

                            self.state.screen.set(Screen::Lobby);
                        }
                    });
                    if ui.button("Back").clicked() {
                        self.state.screen.set(Screen::MainMenu(Submenu::Root));
                    }
                    ui.add_space(BORDER);
                });
            }
            Submenu::Join => {
                TopBottomPanel::top("Title").show(ctx, |ui| {
                    ui.label("Join Game");
                });
                TopBottomPanel::bottom("Host Panel").show(ctx, |ui| {
                    ui.add(TextEdit::singleline(&mut self.player_name).hint_text("Name"));
                    ui.add(
                        TextEdit::singleline(&mut self.lobby_id)
                            .hint_text("Lobby ID")
                            .password(true),
                    );

                    let join_button = ui
                        .add_enabled(self.lobby_id.is_valid(), Button::new("Join Game"))
                        .on_disabled_hover_text("Lobby ID must be an 8 digit hexadecimal number");
                    if join_button.clicked() {
                        self.network_sender
                            .blocking_send(Message::GameJoin {
                                lobby_id: *self.lobby_id.get_val().unwrap(),
                            })
                            .unwrap();
                        self.network_sender
                            .blocking_send(Message::PlayerOptions {
                                options: PlayerOptions {
                                    name: self.player_name.clone(),
                                    color: (0, 0, 0),
                                },
                            })
                            .unwrap();
                        self.state.screen.set(Screen::Lobby);
                    }

                    if ui.button("Back").clicked() {
                        self.state.screen.set(Screen::MainMenu(Submenu::Root));
                    }
                    ui.add_space(BORDER);
                });
            }
        }
    }
}
