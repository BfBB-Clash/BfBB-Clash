use std::borrow::Cow;
use std::mem::ManuallyDrop;
use std::rc::Rc;
use std::thread::JoinHandle;

use clash_lib::lobby::{GamePhase, NetworkedLobby};
use clash_lib::net::{LobbyMessage, Message};
use clash_lib::PlayerId;
use eframe::egui::{Align, Button, CentralPanel, Layout, SidePanel, Ui};
use eframe::App;
use itertools::intersperse;
use tracing::instrument;

use crate::game::ShutdownSender;
use crate::gui::state::State;
use crate::gui::PADDING;
use crate::net::{NetCommand, NetCommandSender};
use player_ui::PlayerUi;
use tracker::Tracker;

use super::handle::{GuiMessage, GuiReceiver};
use super::main_menu::MainMenu;
use super::option_editor::OptionEditor;
use super::val_text::ValText;

mod player_ui;
mod tracker;

#[derive(Debug)]
pub struct LobbyData {
    pub network_sender: NetCommandSender,
    pub gui_receiver: GuiReceiver,
    pub game_shutdown: ManuallyDrop<ShutdownSender>,
    pub network_thread: ManuallyDrop<tokio::task::JoinHandle<()>>,
    pub game_thread: ManuallyDrop<JoinHandle<()>>,
}

impl Drop for LobbyData {
    fn drop(&mut self) {
        // Shutdown game and network threads and wait for them to complete
        self.network_sender
            .blocking_send(NetCommand::Disconnect)
            .expect("Failed to signal network thread to shutdown.");

        // SAFETY: We are dropping ourselves now, so these fields will never be accessed again.
        let (game_shutdown, network_thread, game_thread) = unsafe {
            (
                ManuallyDrop::take(&mut self.game_shutdown),
                ManuallyDrop::take(&mut self.network_thread),
                ManuallyDrop::take(&mut self.game_thread),
            )
        };
        // We want to await the network task to avoid a situation where the network fails to shutdown,
        // but the app seemingly continues as normal
        let stop_net = crate::net::spawn_promise(async move {
            network_thread.await.expect("Network thread failed to join");
        });
        game_shutdown
            .send(())
            .expect("Failed to signal game-logic thread to shutdown");
        game_thread
            .join()
            .expect("Game logic thread failed to join");
        stop_net.block_until_ready();
    }
}

pub struct Game {
    state: Rc<State>,
    lobby_data: LobbyData,
    lobby: NetworkedLobby,
    local_player_id: PlayerId,
    is_host: bool,
    lab_door_cost: ValText<u8>,
    tier_count: ValText<u8>,
    scores: Vec<ValText<u32>>,
}

impl Game {
    pub fn new(state: Rc<State>, lobby_data: LobbyData) -> Self {
        Self {
            state,
            lobby_data,
            lobby: NetworkedLobby::new(0),
            local_player_id: 0.into(),
            is_host: false,
            lab_door_cost: ValText::with_validator(|text| {
                text.parse::<u8>().ok().filter(|&n| n > 0 && n <= 82)
            }),
            tier_count: ValText::with_validator(|text| {
                text.parse::<u8>()
                    .ok()
                    .filter(|&n| n > 0 && n <= clash_lib::MAX_PLAYERS as u8)
            }),
            scores: Default::default(),
        }
    }
}

impl App for Game {
    #[instrument(skip_all)]
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // Receive gamestate updates
        while let Ok(msg) = self.lobby_data.gui_receiver.try_recv() {
            match msg {
                GuiMessage::LocalPlayer(id) => {
                    self.local_player_id = id;
                }
                GuiMessage::LobbyUpdate(new_lobby) => {
                    self.is_host = new_lobby.host_id == Some(self.local_player_id);
                    self.lab_door_cost.set_val(new_lobby.options.lab_door_cost);
                    self.tier_count.set_val(new_lobby.options.tier_count);
                    self.scores
                        .resize_with(new_lobby.options.tier_count as usize, ValText::default);
                    for (i, buf) in self.scores.iter_mut().enumerate() {
                        buf.set_val(new_lobby.options.spat_scores[i]);
                    }

                    self.lobby = new_lobby;
                }
            }
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
                }
                GamePhase::Playing => {
                    Tracker::new(&self.state, &self.lobby, self.local_player_id).ui(ui);
                    ui.vertical_centered(|ui| {
                        if ui.button("Reset").clicked() {
                            self.lobby_data
                                .network_sender
                                .try_send(LobbyMessage::ResetLobby.into())
                                .unwrap();
                        }
                    });
                }
                GamePhase::Finished => self.paint_end(ui),
            }
            ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                ui.with_layout(Layout::right_to_left(Align::BOTTOM), |ui| {
                    if ui
                        .button("Lobby ID")
                        .on_hover_text("Copy Lobby ID to Clipboard")
                        .clicked()
                    {
                        ctx.output().copied_text = format!("{:X}", self.lobby.lobby_id.0);
                    }
                    if ui.button("Leave").clicked() {
                        self.state.change_app(MainMenu::new(self.state.clone()));
                    }
                });
            });
        });
    }
}

impl Game {
    fn paint_options(&mut self, ui: &mut Ui) {
        ui.heading("Lobby Options");
        ui.separator();

        self.options_controls(ui);
        if self.is_host {
            self.host_controls(ui);
        }
    }

    fn options_controls(&mut self, ui: &mut Ui) {
        let mut updated_options = Cow::Borrowed(&self.lobby.options);

        ui.add(
            OptionEditor::new("New Game+", updated_options.ng_plus, |x| {
                updated_options.to_mut().ng_plus = x;
            })
            .enabled(self.is_host),
        )
        .on_hover_text(
            "All players start the game with the Bubble Bowl and Cruise Missile unlocked.",
        );

        ui.add(
            OptionEditor::new("Lab Door Cost", &mut self.lab_door_cost, |n| {
                updated_options.to_mut().lab_door_cost = n;
            })
            .enabled(self.is_host),
        )
        .on_hover_text("Spatulas required to enter Chum Bucket Labs");

        ui.collapsing("Debug Options", |ui| {
            ui.add(
                OptionEditor::new("Tier Count", &mut self.tier_count, |n| {
                    updated_options.to_mut().tier_count = n;
                })
                .enabled(self.is_host),
            )
            .on_hover_text("Number of times a spatula can be collected before it's disabled");

            ui.add(
                OptionEditor::new("Scores", self.scores.as_mut_slice(), |(i, x)| {
                    updated_options.to_mut().spat_scores[i] = x;
                })
                .enabled(self.is_host),
            );
        })
        .header_response
        .on_hover_text("Options that may be revised or removed in the future.");

        if let Cow::Owned(options) = updated_options {
            self.lobby_data
                .network_sender
                .blocking_send(NetCommand::Send(Message::Lobby(
                    LobbyMessage::GameOptions { options },
                )))
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
                        self.lobby
                            .players
                            .values()
                            .filter(|p| !p.ready_to_start)
                            .map(|p| p.options.name.as_str()),
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
            self.lobby_data
                .network_sender
                .try_send(NetCommand::Send(Message::Lobby(LobbyMessage::GameBegin)))
                .unwrap();
        }
    }

    fn paint_end(&mut self, ui: &mut Ui) {
        Tracker::new(&self.state, &self.lobby, self.local_player_id).ui(ui);

        ui.vertical_centered(|ui| {
            // Calculate who won the game, eventually this may be determined by the server or logic thread,
            // as it may differ by gamemode
            // TODO: Handle tie-breaker
            if let Some(winner) = self.lobby.players.values().max_by_key(|&p| p.score) {
                ui.label(format!("{} Wins!", winner.options.name));
            }

            if ui.button("Reset").clicked() {
                self.lobby_data
                    .network_sender
                    .try_send(LobbyMessage::ResetLobby.into())
                    .unwrap();
            }
        });
    }
}
