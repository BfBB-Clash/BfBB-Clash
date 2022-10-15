use std::collections::HashSet;

use bfbb::game_interface::dolphin::DolphinInterface;
use bfbb::game_interface::game_var::{GameVar, GameVarMut};
use bfbb::game_interface::{InterfaceError, InterfaceProvider, InterfaceResult};
use bfbb::game_state::{GameMode as BfBBGameMode, GameOstrich};
use bfbb::{IntoEnumIterator, Level, Spatula};
use clash_lib::lobby::{GamePhase, NetworkedLobby};
use clash_lib::net::{Item, LobbyMessage, Message};
use clash_lib::PlayerId;
use log::info;

use crate::gui::GuiSender;
use crate::net::{NetCommand, NetCommandSender};

use super::game_mode::GameMode;

pub struct ClashGame {
    provider: DolphinInterface,
    lobby: NetworkedLobby,
    /// Not sure if this is the best approach, the idea is that it would be faster
    /// to store a local state of what we as a client have collected rather
    /// than searching to see if we've collected it.
    local_spat_state: HashSet<Spatula>,
    player_id: PlayerId,
}

impl ClashGame {
    pub fn new(player_id: PlayerId) -> Self {
        Self {
            provider: DolphinInterface::default(),
            // FIXME: would be better to be able to construct this object after receiving the initial lobby from the sever
            // but that would complicate the logic thread logic a lot since the initial lobby is received as a regular LobbyMessage::GameLobbyInfo
            lobby: NetworkedLobby::new(0),
            local_spat_state: HashSet::new(),
            player_id,
        }
    }
}

impl GameMode for ClashGame {
    /// Process state updates from the server and report back any actions of the local player
    fn update(&mut self, network_sender: &NetCommandSender) -> InterfaceResult<()> {
        self.provider.do_with_interface(|interface| {
            if interface.is_loading.get()? {
                return Ok(());
            }

            // TODO: Use a better error
            // Find the local player within the lobby
            let local_player = self
                .lobby
                .players
                .get_mut(&self.player_id)
                .ok_or(InterfaceError::Other)?;

            // Detect level changes
            let level = Some(interface.get_current_level()?);
            if local_player.current_level != level {
                local_player.current_level = level;
                network_sender
                    .try_send(NetCommand::Send(Message::Lobby(
                        LobbyMessage::GameCurrentLevel { level },
                    )))
                    .unwrap();
            }

            // We need to be on the title screen to start a new game
            // but we can't if the Demo FMV is active, so we also check that we're currently in a scene.
            let can_start = interface.game_mode.get()? == BfBBGameMode::Title
                && interface.game_ostrich.get()? == GameOstrich::InScene;
            if local_player.ready_to_start != can_start {
                local_player.ready_to_start = can_start;
                network_sender
                    .try_send(NetCommand::Send(Message::Lobby(
                        LobbyMessage::PlayerCanStart(can_start),
                    )))
                    .unwrap();
            }

            // Don't proceed if the game is not active
            if self.lobby.game_phase != GamePhase::Playing || level == Some(Level::MainMenu) {
                return Ok(());
            }

            // Set the cost to unlock the lab door
            interface.set_lab_door(
                self.lobby.options.lab_door_cost.into(),
                local_player.current_level,
            )?;

            // Check for newly collected spatulas
            for spat in Spatula::iter() {
                // Skip already collected spatulas
                if self.local_spat_state.contains(&spat) {
                    interface.mark_task_complete(spat)?;
                    interface.collect_spatula(spat, local_player.current_level)?;
                    continue;
                }

                if let Some(spat_ref) = self.lobby.game_state.spatulas.get_mut(&spat) {
                    if spat_ref.collection_vec.len() != self.lobby.options.tier_count.into() {
                        interface.unlock_task(spat)?;
                    } else {
                        // Sync collected spatulas
                        interface.collect_spatula(spat, local_player.current_level)?;
                        interface.mark_task_complete(spat)?;
                        continue;
                    }
                }

                // Check menu for any potentially missed collection events
                // This is the only way to detect Kah-Rah-Tae and Small Shall Rule
                if interface.is_task_complete(spat)? {
                    self.local_spat_state.insert(spat);
                    network_sender
                        .try_send(NetCommand::Send(Message::Lobby(
                            LobbyMessage::GameItemCollected {
                                item: Item::Spatula(spat),
                            },
                        )))
                        .unwrap();
                    info!("Collected (from menu) {spat:?}");
                }

                // Detect spatula collection events
                if interface.is_spatula_being_collected(spat, local_player.current_level)? {
                    self.local_spat_state.insert(spat);
                    network_sender
                        .try_send(NetCommand::Send(Message::Lobby(
                            LobbyMessage::GameItemCollected {
                                item: Item::Spatula(spat),
                            },
                        )))
                        .unwrap();
                    info!("Collected {spat:?}");
                }
            }

            Ok(())
        })
    }

    fn message(&mut self, message: LobbyMessage, gui_sender: &mut GuiSender) {
        match message {
            LobbyMessage::GameBegin => {
                self.local_spat_state.clear();
                let lobby = &mut self.lobby;

                let _ = self.provider.do_with_interface(|i| {
                    i.powers.start_with_powers(lobby.options.ng_plus)?;
                    i.start_new_game()
                });
                gui_sender
                    .send((self.player_id, lobby.clone()))
                    .expect("GUI has crashed and so will we");
            }
            LobbyMessage::GameLobbyInfo { lobby: new_lobby } => {
                // This could fail if the user is restarting dolphin, but that will desync a lot of other things as well
                // so it's fine to just wait for a future lobby update to correct the issue
                let _ = self.provider.do_with_interface(|i| {
                    i.spatula_count
                        .set(new_lobby.game_state.spatulas.len() as u32)
                });
                self.lobby = new_lobby.clone();
                gui_sender
                    .send((self.player_id, new_lobby))
                    .expect("GUI has crashed and so will we");
            }
            // We're not yet doing partial updates
            LobbyMessage::PlayerOptions { options: _ } => todo!(),
            LobbyMessage::GameOptions { options: _ } => todo!(),
            LobbyMessage::GameEnd => todo!(),
            LobbyMessage::PlayerCanStart(_) => todo!(),
            LobbyMessage::GameCurrentLevel { level: _ } => todo!(),
            LobbyMessage::GameItemCollected { item: _ } => todo!(),
        }
    }
}
