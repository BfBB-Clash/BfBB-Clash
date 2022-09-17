use std::collections::HashSet;

use bfbb::game_interface::{GameInterface, InterfaceError, InterfaceResult};
use bfbb::game_state::{GameMode as BfBBGameMode, GameOstrich};
use bfbb::{IntoEnumIterator, Level, Spatula};
use clash_lib::lobby::{GamePhase, NetworkedLobby};
use clash_lib::net::{Item, Message};
use clash_lib::PlayerId;
use log::info;

use super::game_mode::GameMode;

pub struct ClashGame;

impl GameMode for ClashGame {
    /// Process state updates from the server and report back any actions of the local player
    type Result = InterfaceResult<()>;

    fn update<G: GameInterface>(
        &mut self,
        interface: &G,
        lobby: &mut NetworkedLobby,
        local_player: PlayerId,
        network_sender: &mut tokio::sync::mpsc::Sender<Message>,
        local_spat_state: &mut HashSet<Spatula>,
    ) -> Self::Result {
        if interface.is_loading()? {
            return Ok(());
        }

        // TODO: Use a better error
        // Find the local player within the lobby
        let local_player = lobby
            .players
            .get_mut(&local_player)
            .ok_or(InterfaceError::Other)?;

        // Detect level changes
        let level = Some(interface.get_current_level()?);
        if local_player.current_level != level {
            local_player.current_level = level;
            network_sender
                .try_send(Message::GameCurrentLevel { level })
                .unwrap();
        }

        // We need to be on the title screen to start a new game
        // but we can't if the Demo FMV is active, so we also check that we're currently in a scene.
        let can_start = interface.get_current_game_mode()? == BfBBGameMode::Title
            && interface.get_current_game_ostrich()? == GameOstrich::InScene;
        if local_player.ready_to_start != can_start {
            local_player.ready_to_start = can_start;
            network_sender
                .blocking_send(Message::PlayerCanStart(can_start))
                .unwrap();
        }

        // Don't proceed if the game is not active
        if lobby.game_phase != GamePhase::Playing || level == Some(Level::MainMenu) {
            return Ok(());
        }

        // Set the cost to unlock the lab door
        interface.set_lab_door(
            lobby.options.lab_door_cost.into(),
            local_player.current_level,
        )?;

        // Check for newly collected spatulas
        for spat in Spatula::iter() {
            // Skip already collected spatulas
            if local_spat_state.contains(&spat) {
                interface.mark_task_complete(spat)?;
                interface.collect_spatula(spat, local_player.current_level)?;
                continue;
            }

            if let Some(spat_ref) = lobby.game_state.spatulas.get_mut(&spat) {
                if spat_ref.collection_count != lobby.options.tier_count {
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
                local_spat_state.insert(spat);
                network_sender
                    .try_send(Message::GameItemCollected {
                        item: Item::Spatula(spat),
                    })
                    .unwrap();
                info!("Collected (from menu) {spat:?}");
            }

            // Detect spatula collection events
            if interface.is_spatula_being_collected(spat, local_player.current_level)? {
                local_spat_state.insert(spat);
                network_sender
                    .try_send(Message::GameItemCollected {
                        item: Item::Spatula(spat),
                    })
                    .unwrap();
                info!("Collected {spat:?}");
            }
        }

        Ok(())
    }
}
