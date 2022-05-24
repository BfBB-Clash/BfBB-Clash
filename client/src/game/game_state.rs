use std::collections::HashSet;

use crate::game::{GameInterface, InterfaceResult};
use bfbb::{IntoEnumIterator, Spatula};
use clash::game_state::SpatulaTier;
use clash::lobby::{GamePhase, NetworkedLobby};
use clash::net::{Item, Message};
use clash::PlayerId;
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
        if lobby.game_phase != GamePhase::Playing {
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
                interface.collect_spatula(spat, local_player.current_level)?;
                continue;
            }

            if let Some(spat_ref) = lobby.game_state.spatulas.get_mut(&spat) {
                if spat_ref.tier != SpatulaTier::Golden {
                    interface.mark_task_complete(spat)?;
                }
                if spat_ref.tier == SpatulaTier::None {
                    if local_player.current_level == Some(spat.get_level()) {
                        // Sync collected spatulas
                        interface.collect_spatula(spat, local_player.current_level)?;
                    }
                    continue;
                }
            }
            /*
            // Check menu for any potentially missed collection events
            if game.is_task_complete(spat)? && !hack {
                local_spat_state.insert(spat);
                network_sender
                    .blocking_send(Message::GameItemCollected {
                        item: Item::Spatula(spat),
                    })
                    .unwrap();
                info!("Collected (from menu) {spat:?}");
            }
            */

            // Skip spatulas that aren't in the current room
            if local_player.current_level != Some(spat.get_level()) {
                continue;
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
