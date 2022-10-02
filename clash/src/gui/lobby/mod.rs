use std::mem::ManuallyDrop;
use std::rc::Rc;
use std::thread::JoinHandle;

use clash_lib::lobby::{GamePhase, NetworkedLobby};
use clash_lib::net::{LobbyMessage, Message};
use clash_lib::PlayerId;
use eframe::egui::{
    Align, Button, CentralPanel, Layout, Response, SidePanel, Ui, Widget, WidgetText,
};
use eframe::epaint::Color32;
use eframe::App;
use itertools::intersperse;

use crate::game::ShutdownSender;
use crate::gui::state::State;
use crate::gui::PADDING;
use crate::net::{NetCommand, NetCommandSender};
use player_ui::PlayerUi;
use tracker::Tracker;

use super::main_menu::MainMenu;
use super::val_text::ValText;
use super::GuiReceiver;

mod player_ui;
mod tracker;

#[derive(Debug)]
pub struct LobbyData {
    pub network_sender: NetCommandSender,
    pub gui_receiver: GuiReceiver,
    pub game_shutdown: ManuallyDrop<ShutdownSender>,
    pub network_thread: ManuallyDrop<JoinHandle<()>>,
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
        game_shutdown
            .send(())
            .expect("Failed to signal game-logic thread to shutdown");
        network_thread
            .join()
            .expect("Network thread failed to join");
        game_thread
            .join()
            .expect("Game logic thread failed to join");
    }
}

pub struct Game {
    state: Rc<State>,
    lobby_data: LobbyData,
    lobby: NetworkedLobby,
    local_player_id: PlayerId,
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
            local_player_id: 0,
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
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // Receive gamestate updates
        while let Ok((local_player_id, new_lobby)) = self.lobby_data.gui_receiver.try_recv() {
            self.local_player_id = local_player_id;
            self.lab_door_cost.set_val(new_lobby.options.lab_door_cost);
            self.tier_count.set_val(new_lobby.options.tier_count);
            self.scores
                .resize_with(new_lobby.options.tier_count as usize, ValText::default);
            for (i, buf) in self.scores.iter_mut().enumerate() {
                buf.set_val(new_lobby.options.spat_scores[i]);
            }

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
                    self.state.change_app(MainMenu::new(self.state.clone()));
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

        let mut ng_plus = self.lobby.options.ng_plus;
        ui.add_option("New Game+", &mut ng_plus, |&x| {
            updated_options
                .get_or_insert_with(|| self.lobby.options.clone())
                .ng_plus = x;
        })
        .on_hover_text(
            "All players start the game with the Bubble Bowl and Cruise Missile unlocked.",
        );

        ui.add_option("Lab Door Cost", &mut self.lab_door_cost, |&n| {
            updated_options
                .get_or_insert_with(|| self.lobby.options.clone())
                .lab_door_cost = n;
        })
        .on_hover_text("Spatulas required to enter Chum Bucket Labs");

        ui.collapsing("Debug Options", |ui| {
            ui.add_option("Tier Count", &mut self.tier_count, |&n| {
                updated_options
                    .get_or_insert_with(|| self.lobby.options.clone())
                    .tier_count = n;
            })
            .on_hover_text("Number of times a spatula can be collectd before it's disabled");

            ui.add_option("Scores", self.scores.as_mut_slice(), |&(i, x)| {
                updated_options
                    .get_or_insert_with(|| self.lobby.options.clone())
                    .spat_scores[i] = x;
            });
        });

        if let Some(options) = updated_options {
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
            self.lobby_data
                .network_sender
                .try_send(NetCommand::Send(Message::Lobby(LobbyMessage::GameBegin {})))
                .unwrap();
        }
    }
}

trait UiExt<'a, I: ?Sized, V> {
    fn add_option(
        &mut self,
        label: impl Into<WidgetText>,
        input: &'a mut I,
        on_changed: impl FnMut(&V) + 'a,
    ) -> Response;
}

impl<'a, I, V> UiExt<'a, I, V> for Ui
where
    I: 'a + ?Sized,
    V: 'a,
    OptionEditor<'a, I, V>: Widget,
{
    fn add_option(
        &mut self,
        text: impl Into<WidgetText>,
        val: &'a mut I,
        on_changed: impl FnMut(&V) + 'a,
    ) -> Response {
        let editor = OptionEditor::new(text, val, on_changed);
        self.add(editor)
    }
}

struct OptionEditor<'a, I: ?Sized, V> {
    label: WidgetText,
    input: &'a mut I,
    changed: Box<dyn FnMut(&V) + 'a>,
}

impl<'a, I: ?Sized, V> OptionEditor<'a, I, V> {
    fn new(text: impl Into<WidgetText>, input: &'a mut I, changed: impl FnMut(&V) + 'a) -> Self {
        Self {
            label: text.into(),
            input,
            changed: Box::new(changed),
        }
    }
}

impl<'a, V> Widget for OptionEditor<'a, ValText<V>, V> {
    fn ui(mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.horizontal(|ui| {
            if !self.input.is_valid() {
                ui.style_mut().visuals.override_text_color = Some(Color32::DARK_RED);
            }
            ui.label(self.label);
            if ui.text_edit_singleline(self.input).changed() && self.input.is_valid() {
                if let Some(x) = self.input.get_val() {
                    (self.changed)(x);
                }
            }
        })
        .response
    }
}

impl<'a> Widget for OptionEditor<'a, bool, bool> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            if ui.checkbox(self.input, self.label).changed() {
                (self.changed)(self.input);
            }
        })
        .response
    }
}

impl<'a, T: Copy> Widget for OptionEditor<'a, [ValText<T>], (usize, T)> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        ui.collapsing(self.label, |ui| {
            for (i, input) in self.input.iter_mut().enumerate() {
                ui.add_option(format!("Tier {}", i + 1), input, |&x| {
                    (self.changed)(&(i, x))
                });
            }
        })
        .header_response
    }
}
