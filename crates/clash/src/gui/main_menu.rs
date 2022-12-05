use std::{mem::ManuallyDrop, rc::Rc};

use clash_lib::{
    net::{LobbyMessage, Message},
    player::PlayerOptions,
    LobbyId,
};
use eframe::{
    egui::{Align, Button, CentralPanel, Layout, TextEdit, TopBottomPanel},
    App,
};
use tracing::instrument;

use crate::gui::BORDER;
use crate::{
    game,
    gui::state::State,
    net::{self, NetCommand},
};

use super::{
    handle::GuiHandle,
    lobby::{Game, LobbyData},
    val_text::ValText,
};

pub struct MainMenu {
    state: Rc<State>,
    submenu: Submenu,
    player_name: String,
    lobby_id: ValText<LobbyId>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Submenu {
    Root,
    Host,
    Join,
}

impl MainMenu {
    pub fn new(state: Rc<State>) -> Self {
        Self {
            state,
            submenu: Submenu::Root,
            player_name: Default::default(),
            // TODO: atm it is not strictly true that the lobby_id must be 8 digits,
            //  since it's just a random u32. When this is resolved on the server-side,
            //  we need to validate it here as well.
            lobby_id: ValText::with_validator(|text| {
                u32::from_str_radix(text.strip_prefix("0x").unwrap_or(text), 16)
                    .map(|v| v.into())
                    .ok()
            }),
        }
    }
}

impl App for MainMenu {
    #[instrument(skip_all)]
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
                TopBottomPanel::bottom("Host Panel").show(ctx, |ui| {
                    ui.add(TextEdit::singleline(&mut self.player_name).hint_text("Name"));
                    ui.add_enabled_ui(!self.player_name.is_empty(), |ui| {
                        let host_button = ui
                            .button("Host Game")
                            .on_disabled_hover_text("Player Name is required");
                        if host_button.clicked() {
                            let lobby_data = self.spawn_net(ctx.clone());
                            lobby_data
                                .network_sender
                                .try_send(NetCommand::Send(Message::GameHost))
                                .unwrap();
                            lobby_data
                                .network_sender
                                .try_send(NetCommand::Send(Message::Lobby(
                                    LobbyMessage::PlayerOptions {
                                        options: PlayerOptions {
                                            name: self.player_name.clone(),
                                            color: (0, 0, 0),
                                        },
                                    },
                                )))
                                .unwrap();

                            self.state
                                .change_app(Game::new(self.state.clone(), lobby_data));
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
                TopBottomPanel::bottom("Join Panel").show(ctx, |ui| {
                    ui.add(TextEdit::singleline(&mut self.player_name).hint_text("Name"));
                    ui.add(
                        TextEdit::singleline(&mut self.lobby_id)
                            .hint_text("Lobby ID")
                            .password(true),
                    );

                    ui.horizontal(|ui| {
                        let mut join_button = ui.add_enabled(
                            self.lobby_id.is_valid() && !self.player_name.is_empty(),
                            Button::new("Join Game"),
                        );
                        if !self.lobby_id.is_valid() {
                            join_button = join_button.on_disabled_hover_text(
                                "Lobby ID must be an 8 digit hexadecimal number",
                            );
                        }
                        if self.player_name.is_empty() {
                            join_button =
                                join_button.on_disabled_hover_text("Player Name is required")
                        }
                        if join_button.clicked() {
                            let lobby_data = self.spawn_net(ctx.clone());
                            lobby_data
                                .network_sender
                                .try_send(NetCommand::Send(Message::GameJoin {
                                    lobby_id: self.lobby_id.get_val().unwrap(),
                                    spectate: false,
                                }))
                                .unwrap();
                            lobby_data
                                .network_sender
                                .try_send(NetCommand::Send(Message::Lobby(
                                    LobbyMessage::PlayerOptions {
                                        options: PlayerOptions {
                                            name: self.player_name.clone(),
                                            color: (0, 0, 0),
                                        },
                                    },
                                )))
                                .unwrap();
                            self.state
                                .change_app(Game::new(self.state.clone(), lobby_data));
                        }

                        let spectate_button = ui
                            .add_enabled(self.lobby_id.is_valid(), Button::new("Spectate"))
                            .on_disabled_hover_text(
                                "Lobby ID must be an 8 digit hexadecimal number",
                            );
                        if spectate_button.clicked() {
                            let lobby_data = self.spawn_net(ctx.clone());
                            lobby_data
                                .network_sender
                                .try_send(NetCommand::Send(Message::GameJoin {
                                    lobby_id: self.lobby_id.get_val().unwrap(),
                                    spectate: true,
                                }))
                                .unwrap();
                            self.state
                                .change_app(Game::new(self.state.clone(), lobby_data));
                        }
                    });

                    if ui.button("Back").clicked() {
                        self.submenu = Submenu::Root;
                    }
                    ui.add_space(BORDER);
                });
            }
        }
    }
}

impl MainMenu {
    fn spawn_net(&self, gui_ctx: eframe::egui::Context) -> LobbyData {
        let (network_sender, network_receiver) = tokio::sync::mpsc::channel::<NetCommand>(32);
        let (logic_sender, logic_receiver) = std::sync::mpsc::channel::<Message>();
        // Create a new thread and start a tokio runtime on it for talking to the server
        let error_sender = self.state.error_sender.clone();
        let network_thread = std::thread::Builder::new()
            .name("Network".into())
            .spawn(move || net::run(network_receiver, logic_sender, error_sender))
            .expect("Couldn't start network thread.");

        // Start Game Thread
        let (gui_sender, gui_receiver) = std::sync::mpsc::channel();
        let gui_handle = GuiHandle {
            context: gui_ctx,
            sender: gui_sender,
        };
        let (game_shutdown, shutdown_receiver) = tokio::sync::oneshot::channel();
        let game_thread = {
            let network_sender = network_sender.clone();
            std::thread::Builder::new()
                .name("Logic".into())
                .spawn(move || {
                    game::start_game(
                        gui_handle,
                        network_sender,
                        logic_receiver,
                        shutdown_receiver,
                    )
                })
                .expect("Couldn't start game-logic thread")
        };

        LobbyData {
            network_sender,
            gui_receiver,
            game_shutdown: ManuallyDrop::new(game_shutdown),
            network_thread: ManuallyDrop::new(network_thread),
            game_thread: ManuallyDrop::new(game_thread),
        }
    }
}
