use std::collections::HashSet;

use bfbb::game_interface::game_var::{GameVar, GameVarMut};
use bfbb::game_interface::{InterfaceError, InterfaceProvider, InterfaceResult};
use bfbb::game_state::{GameMode as BfBBGameMode, GameOstrich};
use bfbb::{IntoEnumIterator, Level, Spatula};
use clash_lib::lobby::{GamePhase, NetworkedLobby};
use clash_lib::net::{Item, LobbyMessage, Message};
use clash_lib::PlayerId;
use tracing::instrument;

use crate::gui::handle::GuiHandle;
use crate::net::{NetCommand, NetCommandSender};

use super::game_mode::GameMode;

pub struct ClashGame<I> {
    provider: I,
    lobby: NetworkedLobby,
    /// Not sure if this is the best approach, the idea is that it would be faster
    /// to store a local state of what we as a client have collected rather
    /// than searching to see if we've collected it.
    local_spat_state: HashSet<Spatula>,
    player_id: PlayerId,
}

impl<I> std::fmt::Debug for ClashGame<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // write!(f, "ClashGame")
        f.debug_struct("ClashGame").finish_non_exhaustive()
    }
}

impl<I> ClashGame<I> {
    pub fn new(provider: I, player_id: PlayerId) -> Self {
        Self {
            provider,
            // FIXME: would be better to be able to construct this object after receiving the initial lobby from the sever
            // that's difficult because the client is given their id and their lobby in separate messages.
            lobby: NetworkedLobby::new(0),
            local_spat_state: HashSet::new(),
            player_id,
        }
    }
}

impl<I: InterfaceProvider> GameMode for ClashGame<I> {
    /// Process state updates from the server and report back any actions of the local player
    #[instrument(skip_all, fields(game_mode = ?self))]
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
                    tracing::info!("Collected (from menu) {spat:?}");
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
                    tracing::info!("Collected {spat:?}");
                }
            }

            Ok(())
        })
    }

    fn message(&mut self, message: LobbyMessage, gui_handle: &mut GuiHandle) {
        match message {
            LobbyMessage::GameBegin => {
                self.local_spat_state.clear();
                let lobby = &self.lobby;

                let _ = self.provider.do_with_interface(|i| {
                    i.powers.start_with_powers(lobby.options.ng_plus)?;
                    i.start_new_game()
                });
                gui_handle.send(lobby.clone());
            }
            // We're not yet doing partial updates
            LobbyMessage::ResetLobby => todo!(),
            LobbyMessage::PlayerOptions { options: _ } => todo!(),
            LobbyMessage::GameOptions { options: _ } => todo!(),
            LobbyMessage::GameEnd => todo!(),
            LobbyMessage::PlayerCanStart(_) => todo!(),
            LobbyMessage::GameCurrentLevel { level: _ } => todo!(),
            LobbyMessage::GameItemCollected { item: _ } => todo!(),
        }
    }

    fn update_lobby(&mut self, new_lobby: NetworkedLobby, gui_sender: &mut GuiHandle) {
        // This could fail if the user is restarting dolphin, but that will desync a lot of other things as well
        // so it's fine to just wait for a future lobby update to correct the issue
        let _ = self.provider.do_with_interface(|i| {
            i.spatula_count
                .set(new_lobby.game_state.spatulas.len() as u32)
        });
        self.lobby = new_lobby.clone();
        gui_sender.send(new_lobby);
    }
}

#[cfg(test)]
mod tests {
    use clash_lib::{
        game_state::SpatulaState,
        lobby::{GamePhase, NetworkedLobby},
        net::{LobbyMessage, Message},
        player::{NetworkedPlayer, PlayerOptions},
    };

    use bfbb::{
        game_interface::{
            mock::{mock_vars::MockBackend, MockInterface},
            GameInterface, InterfaceProvider, InterfaceResult,
        },
        Level, Spatula,
    };

    use crate::{game::game_mode::GameMode, gui::handle::GuiHandle, net::NetCommand};

    use super::ClashGame;

    fn setup_game(
        provider_setup: impl FnOnce(&mut GameInterface<MockBackend>) -> InterfaceResult<()>,
    ) -> ClashGame<MockInterface> {
        // Run any required game-state setup code
        let mut provider = MockInterface::default();
        provider.do_with_interface(provider_setup).unwrap();

        // Setup a default lobby state
        let mut lobby = NetworkedLobby::new(0);
        let mut player = NetworkedPlayer::new(PlayerOptions::default(), 0);
        player.current_level = Some(Level::SpongebobHouse);
        lobby.players.insert(0.into(), player);
        lobby.game_phase = GamePhase::Playing;

        // Make new ClashGame and give it the default lobby
        let mut handle = GuiHandle::dummy();
        let mut game = ClashGame::new(provider, 0.into());
        game.update_lobby(lobby, &mut handle);
        game
    }

    fn update_and_check(
        game: &mut ClashGame<MockInterface>,
        expected: impl IntoIterator<Item = LobbyMessage>,
    ) {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(16);
        game.update(&sender).unwrap();
        for e in expected.into_iter() {
            match receiver.try_recv() {
                Ok(NetCommand::Send(Message::Lobby(m))) => assert_eq!(e, m),
                Ok(m) => panic!("Incorrect Message. Got: {m:#?}\nExpected: {e:#?}"),
                Err(_) => panic!("No message available. Expected message {e:#?}"),
            }
        }
    }

    #[test]
    fn spat_get_from_object() {
        let mut game = setup_game(|interface| {
            let task = &mut interface.tasks[Spatula::SpongebobsCloset];
            task.state.as_mut().unwrap().value |= 4;
            Ok(())
        });
        update_and_check(
            &mut game,
            Some(LobbyMessage::GameItemCollected {
                item: Spatula::SpongebobsCloset.into(),
            }),
        );
    }

    #[test]
    fn spat_get_from_menu() {
        let mut game = setup_game(|interface| {
            let task = &mut interface.tasks[Spatula::OnTopOfThePineapple];
            task.menu_count.value = 2;
            Ok(())
        });
        update_and_check(
            &mut game,
            Some(LobbyMessage::GameItemCollected {
                item: Spatula::OnTopOfThePineapple.into(),
            }),
        );
    }

    #[test]
    fn no_collecting_when_loading() {
        let mut game = setup_game(|interface| {
            interface.is_loading.value = true;
            let task = &mut interface.tasks[Spatula::OnTopOfThePineapple];
            task.state.as_mut().unwrap().value |= 4;
            task.menu_count.value = 2;
            Ok(())
        });
        update_and_check(&mut game, None);
    }

    #[test]
    fn no_collecting_when_on_menu() {
        let mut game = setup_game(|interface| {
            interface.scene_id.value = Level::MainMenu.into();
            let task = &mut interface.tasks[Spatula::OnTopOfThePineapple];
            task.state.as_mut().unwrap().value |= 4;
            Ok(())
        });
        // Make sure no GameItemCollected message is emmitted.
        update_and_check(
            &mut game,
            Some(LobbyMessage::GameCurrentLevel {
                level: Some(Level::MainMenu),
            }),
        );

        // Also ensure that we don't sync state while on the main menu
        game.lobby.game_state.spatulas.insert(
            Spatula::CowaBungee,
            SpatulaState {
                collection_vec: vec![1.into()],
            },
        );
        update_and_check(&mut game, None);
        assert_eq!(game.provider.tasks[Spatula::CowaBungee].menu_count.value, 0);
    }

    #[test]
    fn change_level() {
        let mut game = setup_game(|interface| {
            interface.scene_id.value = Level::JellyfishRock.into();
            Ok(())
        });
        update_and_check(
            &mut game,
            Some(LobbyMessage::GameCurrentLevel {
                level: Some(Level::JellyfishRock),
            }),
        )
    }

    #[test]
    fn ng_plus_disables() {
        let mut game = setup_game(|interface| {
            interface.scene_id.value = Level::MainMenu.into();
            Ok(())
        });
        game.lobby.options.ng_plus = true;

        let mut handle = GuiHandle::dummy();
        game.message(LobbyMessage::GameBegin, &mut handle);
        assert!(game.provider.powers.initial_bubble_bowl.value);
        assert!(game.provider.powers.initial_cruise_bubble.value);

        game.lobby.options.ng_plus = false;
        game.message(LobbyMessage::GameBegin, &mut handle);
        assert!(!game.provider.powers.initial_bubble_bowl.value);
        assert!(!game.provider.powers.initial_cruise_bubble.value);
    }
}
