use std::collections::HashSet;

use bfbb::game_interface::{GameInterface, InterfaceResult};
use bfbb::{IntoEnumIterator, Level, Spatula};
use clash_lib::game_state::SpatulaTier;
use clash_lib::lobby::{GamePhase, NetworkedLobby};
use clash_lib::net::{Item, Message};
use clash_lib::PlayerId;
use log::info;

use crate::game::InterfaceError;

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
                .blocking_send(Message::GameCurrentLevel { level })
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
                if spat_ref.tier != SpatulaTier::Golden {
                    interface.unlock_task(spat)?;
                }
                if spat_ref.tier == SpatulaTier::None {
                    // Sync collected spatulas
                    interface.collect_spatula(spat, local_player.current_level)?;
                    continue;
                }
            }

            // Check menu for any potentially missed collection events
            // This is the only way to detect Kah-Rah-Tae and Small Shall Rule
            if interface.is_task_complete(spat)? {
                local_spat_state.insert(spat);
                network_sender
                    .blocking_send(Message::GameItemCollected {
                        item: Item::Spatula(spat),
                    })
                    .unwrap();
                info!("Collected (from menu) {spat:?}");
            }

            // Detect spatula collection events
            if interface.is_spatula_being_collected(spat, local_player.current_level)? {
                local_spat_state.insert(spat);
                network_sender
                    .blocking_send(Message::GameItemCollected {
                        item: Item::Spatula(spat),
                    })
                    .unwrap();
                info!("Collected {spat:?}");
            }
        }

        Ok(())
    }
}
