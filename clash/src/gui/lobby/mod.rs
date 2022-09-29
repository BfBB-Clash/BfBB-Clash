use std::{rc::Rc, sync::mpsc::Receiver};

use clash_lib::lobby::{GamePhase, NetworkedLobby};
use clash_lib::net::Message;
use clash_lib::PlayerId;
use eframe::egui::{Align, Button, CentralPanel, Checkbox, Layout, SidePanel, Ui};
use eframe::epaint::Color32;
use eframe::App;
use itertools::intersperse;

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
    lab_door_cost: ValText<u8>,
    tier_count: ValText<u8>,
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
            lab_door_cost: ValText::with_validator(|text| {
                text.parse::<u8>().ok().filter(|&n| n > 0 && n <= 82)
            }),
            tier_count: ValText::with_validator(|text| {
                text.parse::<u8>()
                    .ok()
                    .filter(|&n| n > 0 && n <= clash_lib::MAX_PLAYERS as u8)
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
            self.lab_door_cost.set_val(new_lobby.options.lab_door_cost);
            self.tier_count.set_val(new_lobby.options.tier_count);
            self.lobby = new_lobby;
        }

        SidePanel::left("Player List")
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(PADDING);
                // TODO: Cache this
                let mut players = self.lobby.players.values().collect::<Vec<_>>();
                players.sort_by(|&a, &b| a.menu_order.cmp(&b.menu_order));
                for player in players {
                    ui.add(PlayerUi::new(player));
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
                    ui.add(Tracker::new(&self.lobby, self.local_player_id));
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
            self.options_controls(ui);
            if ui.is_enabled() {
                self.host_controls(ui);
            }
        });
    }

    fn options_controls(&mut self, ui: &mut Ui) {
        let mut updated_options = None;

        if ui
            .add(Checkbox::new(&mut self.lobby.options.ng_plus, "New Game+"))
            .on_hover_text(
                "All players start the game with the Bubble Bowl and Cruise Missile unlocked.",
            )
            .changed()
        {
            updated_options
                .get_or_insert_with(|| self.lobby.options.clone())
                .ng_plus = self.lobby.options.ng_plus;
        }

        let lab_door_ui = ui
            .horizontal(|ui| {
                if !self.lab_door_cost.is_valid() {
                    ui.style_mut().visuals.override_text_color = Some(Color32::DARK_RED);
                }
                ui.label("Lab Door Cost: ");
                ui.text_edit_singleline(&mut self.lab_door_cost)
            })
            .inner;
        if lab_door_ui.changed() {
            if let Some(&n) = self.lab_door_cost.get_val() {
                updated_options
                    .get_or_insert_with(|| self.lobby.options.clone())
                    .lab_door_cost = n;
            }
        }

        let tier_ui = ui
            .horizontal(|ui| {
                if !self.tier_count.is_valid() {
                    ui.style_mut().visuals.override_text_color = Some(Color32::DARK_RED);
                }
                ui.label("Tier Count: ");
                ui.text_edit_singleline(&mut self.tier_count)
            })
            .inner;
        if tier_ui.changed() {
            if let Some(&n) = self.tier_count.get_val() {
                updated_options
                    .get_or_insert_with(|| self.lobby.options.clone())
                    .tier_count = n;
            }
        }

        if let Some(options) = updated_options {
            self.network_sender
                .blocking_send(Message::GameOptions { options })
                .unwrap();
        }
    }

    fn host_controls(&mut self, ui: &mut Ui) {
        let mut start_game_response = ui
            .add_enabled(
                self.lobby.can_start()
                    && self.lab_door_cost.is_valid()
                    && self.tier_count.is_valid(),
                Button::new("Start Game"),
            )
            .on_hover_text("Starts a new game for all connected players.");

        // We need to check these conditions twice separately so that we only add tooltips for
        // the particular conditons that are preventing the game from starting.
        if !self.lobby.can_start() {
            start_game_response = start_game_response
                .on_disabled_hover_text("All players must be on the Main Menu.")
                .on_disabled_hover_text(format!(
                    "Waiting on: {}",
                    intersperse(
                        self.lobby.players.values().map(|p| p.options.name.as_str()),
                        ", "
                    )
                    .collect::<String>()
                ))
        }

        if !self.lab_door_cost.is_valid() {
            start_game_response = start_game_response
                .on_disabled_hover_text("'Lab Door Cost' must be a number from 1-82");
        }

        if !self.tier_count.is_valid() {
            start_game_response =
                start_game_response.on_disabled_hover_text("'Tier Count' must be a number from 1-6")
        }

        if start_game_response.clicked() {
            // TODO: Send a message to the network thread to start the game.
            self.network_sender
                .blocking_send(Message::GameBegin {})
                .unwrap();
        }
    }
}
