use std::{rc::Rc, sync::mpsc::Receiver};

use clash::{
    lobby::{GamePhase, NetworkedLobby},
    net::Message,
    PlayerId,
};
use eframe::{
    egui::{Align, Button, CentralPanel, Checkbox, Layout, SidePanel, Ui},
    epaint::Color32,
    App,
};

use crate::gui::state::{Screen, State, Submenu};
use crate::gui::PADDING;
use player_ui::PlayerUi;
use tracker::Tracker;

use super::val_text::ValText;

mod player_ui;
mod tracker;

pub struct Game {
    state: Rc<State>,
    gui_receiver: Receiver<(PlayerId, NetworkedLobby)>,
    network_sender: tokio::sync::mpsc::Sender<Message>,
    lobby: NetworkedLobby,
    local_player_id: PlayerId,
    lab_door_num: ValText<u8>,
}

impl Game {
    pub fn new(
        state: Rc<State>,
        gui_receiver: Receiver<(PlayerId, NetworkedLobby)>,
        network_sender: tokio::sync::mpsc::Sender<Message>,
    ) -> Self {
        Self {
            state,
            gui_receiver,
            network_sender,
            lobby: NetworkedLobby::new(0),
            local_player_id: 0,
            lab_door_num: ValText::with_validator(|text| {
                text.parse::<u8>().ok().filter(|&n| n > 0 && n <= 82)
            }),
        }
    }
}

impl App for Game {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // Continuously repaint
        ctx.request_repaint();

        // Receive gamestate updates
        while let Ok((local_player_id, new_lobby)) = self.gui_receiver.try_recv() {
            self.local_player_id = local_player_id;
            self.lab_door_num.set_val(new_lobby.options.lab_door_cost);
            self.lobby = new_lobby;
        }

        SidePanel::left("Player List")
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(PADDING);
                // TODO: Cache this
                let mut values = self.lobby.players.iter().collect::<Vec<_>>();
                values.sort_by(|&a, &b| a.1.menu_order.cmp(&b.1.menu_order));
                for (player_id, player) in values {
                    let score = self
                        .lobby
                        .game_state
                        .scores
                        .get(player_id)
                        .unwrap_or(&0)
                        .clone();
                    ui.add(PlayerUi::new(player, score));
                }
            });
        CentralPanel::default().show(ctx, |ui| {
            match self.lobby.game_phase {
                GamePhase::Setup => {
                    self.paint_options(ui);
                    if ui.button("Copy Lobby ID").clicked() {
                        ctx.output().copied_text = format!("{:X}", self.lobby.lobby_id);
                    }
                }
                GamePhase::Playing => {
                    ui.add(Tracker::new(&self.lobby.game_state, &self.lobby.players));
                }
                GamePhase::Finished => todo!(),
            }
            ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                if ui.button("Leave").clicked() {
                    let _ = self.network_sender.blocking_send(Message::GameLeave);
                    self.state.screen.set(Screen::MainMenu(Submenu::Root));
                }
            })
        });
    }
}

impl Game {
    fn paint_options(&mut self, ui: &mut Ui) {
        ui.heading("Lobby Options");
        ui.separator();

        ui.add_enabled_ui(self.lobby.host_id == Some(self.local_player_id), |ui| {
            let ng_plus_response = ui
                .add(Checkbox::new(&mut self.lobby.options.ng_plus, "New Game+"))
                .on_hover_text(
                    "All players start the game with the Bubble Bowl and Cruise Missile unlocked.",
                );

            let door_cost_response = ui
                .horizontal(|ui| {
                    if !self.lab_door_num.is_valid() {
                        ui.style_mut().visuals.override_text_color = Some(Color32::DARK_RED);
                    }
                    ui.label("Lab Door Cost: ");
                    ui.text_edit_singleline(&mut self.lab_door_num)
                })
                .inner;

            if !ui.is_enabled() {
                return;
            }

            // It shouldn't be possible to change multiple options in one update, so not batching
            // them here shouldn't result in potential to "double update" the server
            if door_cost_response.changed() {
                if let Some(n) = self.lab_door_num.get_val() {
                    self.lobby.options.lab_door_cost = *n;
                    self.network_sender
                        .blocking_send(Message::GameOptions {
                            options: self.lobby.options.clone(),
                        })
                        .unwrap();
                }
            }
            if ng_plus_response.changed() {
                self.network_sender
                    .blocking_send(Message::GameOptions {
                        options: self.lobby.options.clone(),
                    })
                    .unwrap();
            }

            let mut start_game_response = ui
                .add_enabled(
                    self.lobby.can_start() && self.lab_door_num.is_valid(),
                    Button::new("Start Game"),
                )
                .on_hover_text("Starts a new game for all connected players.");

            // We need to check these conditions twice separately so that we only add tooltips for
            // the particular conditons that are preventing the game from starting.
            if !self.lobby.can_start() {
                start_game_response = start_game_response
                    .on_disabled_hover_text("All players must be on the Main Menu.")
            }

            if !self.lab_door_num.is_valid() {
                start_game_response = start_game_response
                    .on_disabled_hover_text("'Lab Door Cost' must be a number from 1-82");
            }

            if start_game_response.clicked() {
                // TODO: Send a message to the network thread to start the game.
                self.network_sender
                    .blocking_send(Message::GameBegin {})
                    .unwrap();
            }
        });
    }
}
