use bfbb::{Level, Spatula};
use clash_lib::game_state::SpatulaState;
use clash_lib::lobby::{GamePhase, LobbyOptions, NetworkedLobby};
use clash_lib::net::{Item, LobbyMessage, Message};
use clash_lib::player::{NetworkedPlayer, PlayerOptions};
use clash_lib::{LobbyId, PlayerId, GAME_CONSTS, MAX_PLAYERS};
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::state::ServerState;

use super::{LobbyError, LobbyResult};

pub struct LobbyActor {
    state: ServerState,
    receiver: mpsc::Receiver<LobbyAction>,
    shared: NetworkedLobby,
    sender: broadcast::Sender<Message>,
    next_menu_order: u8,
}

#[derive(Debug)]
pub enum LobbyAction {
    StartGame {
        respond_to: oneshot::Sender<LobbyResult<()>>,
        id: PlayerId,
    },
    AddPlayer {
        respond_to: oneshot::Sender<LobbyResult<broadcast::Receiver<Message>>>,
        id: PlayerId,
    },
    RemovePlayer {
        id: PlayerId,
    },
    SetPlayerOptions {
        respond_to: oneshot::Sender<LobbyResult<()>>,
        id: PlayerId,
        options: PlayerOptions,
    },
    SetPlayerCanStart {
        respond_to: oneshot::Sender<LobbyResult<()>>,
        id: PlayerId,
        can_start: bool,
    },
    SetPlayerLevel {
        respond_to: oneshot::Sender<LobbyResult<()>>,
        id: PlayerId,
        level: Option<Level>,
    },
    PlayerCollectedItem {
        respond_to: oneshot::Sender<LobbyResult<()>>,
        id: PlayerId,
        item: Item,
    },
    SetGameOptions {
        respond_to: oneshot::Sender<LobbyResult<()>>,
        id: PlayerId,
        options: LobbyOptions,
    },
}

impl LobbyActor {
    pub fn new(
        state: ServerState,
        receiver: mpsc::Receiver<LobbyAction>,
        lobby_id: LobbyId,
    ) -> Self {
        let (sender, _) = broadcast::channel(100);

        Self {
            state,
            receiver,
            shared: NetworkedLobby::new(lobby_id),
            sender,
            next_menu_order: 0,
        }
    }

    pub async fn run(mut self) {
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                LobbyAction::StartGame { respond_to, id } => {
                    let _ = respond_to.send(self.start_game(id));
                }
                LobbyAction::AddPlayer { respond_to, id } => {
                    let _ = respond_to.send(self.add_player(id));
                }
                LobbyAction::RemovePlayer { id } => self.rem_player(id),
                LobbyAction::SetPlayerOptions {
                    respond_to,
                    id,
                    options,
                } => {
                    let _ = respond_to.send(self.set_player_options(id, options));
                }
                LobbyAction::SetPlayerCanStart {
                    respond_to,
                    id,
                    can_start,
                } => {
                    let _ = respond_to.send(self.set_player_can_start(id, can_start));
                }
                LobbyAction::SetPlayerLevel {
                    respond_to,
                    id,
                    level,
                } => {
                    let _ = respond_to.send(self.set_player_level(id, level));
                }
                LobbyAction::PlayerCollectedItem {
                    respond_to,
                    id,
                    item,
                } => {
                    let _ = respond_to.send(self.player_collected_item(id, item));
                }
                LobbyAction::SetGameOptions {
                    respond_to,
                    id,
                    options,
                } => {
                    let _ = respond_to.send(self.set_game_options(id, options));
                }
            }
        }

        // Remove this lobby from the server
        let state = &mut *self.state.lock().unwrap();
        state.lobbies.remove(&self.shared.lobby_id);
        log::info!("Closing lobby {:#X}", self.shared.lobby_id);
    }

    fn reset_game(&mut self) {
        self.shared.game_state.reset_state();
        self.shared
            .players
            .values_mut()
            .for_each(NetworkedPlayer::reset_state);
    }

    fn start_game(&mut self, player_id: PlayerId) -> LobbyResult<()> {
        if self.shared.host_id != Some(player_id) {
            return Err(LobbyError::NeedsHost);
        }

        if !self.shared.can_start() {
            log::warn!(
                "Lobby {:#X} attempted to start when some players aren't able to start.",
                self.shared.lobby_id
            );
            // Maybe this should be an error, I'm not sure
            return Ok(());
        }

        self.reset_game();
        self.shared.game_phase = GamePhase::Playing;
        let _ = self
            .sender
            .send(Message::Lobby(LobbyMessage::GameLobbyInfo {
                lobby: self.shared.clone(),
            }));
        if self
            .sender
            .send(Message::Lobby(LobbyMessage::GameBegin))
            .is_err()
        {
            log::warn!(
                "Lobby {:#X} started with no players in lobby.",
                self.shared.lobby_id
            )
        }

        Ok(())
    }

    fn stop_game(&mut self) {
        self.shared.game_phase = GamePhase::Setup;
        let _ = self
            .sender
            .send(Message::Lobby(LobbyMessage::GameLobbyInfo {
                lobby: self.shared.clone(),
            }));
        if self
            .sender
            .send(Message::Lobby(LobbyMessage::GameEnd))
            .is_err()
        {
            log::warn!(
                "Lobby {:#X} finished with no players in lobby.",
                self.shared.lobby_id
            )
        }
    }

    /// Adds a new player to this lobby. If there is currently no host, they will become it.
    /// A `[broadcast::Receiver]` is returned that will be sent all future events that happen
    /// to this lobby.
    ///
    /// # Errors
    ///
    /// This function will return an error if the lobby is already full
    fn add_player(&mut self, player_id: PlayerId) -> LobbyResult<broadcast::Receiver<Message>> {
        if self.shared.players.len() >= MAX_PLAYERS {
            return Err(LobbyError::LobbyFull);
        }

        // TODO: Unhardcode player color
        let mut player = NetworkedPlayer::new(PlayerOptions::default(), self.next_menu_order);
        player.options.color = clash_lib::player::COLORS[self.shared.players.len()];
        self.next_menu_order += 1;

        self.shared.players.insert(player_id, player);
        // TODO: When the last player in a lobby leaves, it is closed, therefore this should just be
        //  done once when the lobby is first created. (This will also allow us to get rid of the Option
        //  for the lobby's host_id)
        if self.shared.host_id == None {
            self.shared.host_id = Some(player_id);
        }

        // Subscribe early so that this player will receive the lobby update that adds them
        let recv = self.sender.subscribe();

        let _ = self
            .sender
            .send(Message::Lobby(LobbyMessage::GameLobbyInfo {
                lobby: self.shared.clone(),
            }));

        Ok(recv)
    }

    /// Removes a player from the lobby. If the host is removed, a new host is assigned randomly.
    fn rem_player(&mut self, player_id: PlayerId) {
        if self.shared.players.remove(&player_id).is_none() {
            log::warn!(
                "Attempted to remove player {:#} from lobby {:#} who isn't in it",
                player_id,
                self.shared.lobby_id
            );
            return;
        }
        if self.shared.host_id == Some(player_id) {
            // Pass host to first remaining player in list (effectively random with a HashMap)
            // NOTE: We could consider passing host based on join order
            self.shared.host_id = self.shared.players.iter().next().map(|(&id, _)| id);
        }

        // Update remaining clients of the change
        let _ = self
            .sender
            .send(Message::Lobby(LobbyMessage::GameLobbyInfo {
                lobby: self.shared.clone(),
            }));

        // Close the lobby after the last player leaves by closing our receiver.
        // This will cause the run loop to consume all remaining messages,
        // (likely none since the last player just left), and then exit
        if self.shared.players.is_empty() {
            self.receiver.close();
        }
    }

    fn set_player_options(
        &mut self,
        player_id: PlayerId,
        mut options: PlayerOptions,
    ) -> LobbyResult<()> {
        let player = self
            .shared
            .players
            .get_mut(&player_id)
            .ok_or(LobbyError::PlayerInvalid(player_id))?;

        // TODO: Unhardcode player color
        options.color = player.options.color;
        player.options = options;

        let message = Message::Lobby(LobbyMessage::GameLobbyInfo {
            lobby: self.shared.clone(),
        });
        let _ = self.sender.send(message);
        Ok(())
    }

    fn set_player_can_start(&mut self, player_id: PlayerId, can_start: bool) -> LobbyResult<()> {
        let player = self
            .shared
            .players
            .get_mut(&player_id)
            .ok_or(LobbyError::PlayerInvalid(player_id))?;

        player.ready_to_start = can_start;
        let message = Message::Lobby(LobbyMessage::GameLobbyInfo {
            lobby: self.shared.clone(),
        });
        let _ = self.sender.send(message);
        Ok(())
    }

    fn set_player_level(&mut self, player_id: PlayerId, level: Option<Level>) -> LobbyResult<()> {
        let player = self
            .shared
            .players
            .get_mut(&player_id)
            .ok_or(LobbyError::PlayerInvalid(player_id))?;

        player.current_level = level;
        log::info!("Player {:#X} entered {level:?}", player_id);

        let message = Message::Lobby(LobbyMessage::GameLobbyInfo {
            lobby: self.shared.clone(),
        });
        let _ = self.sender.send(message);
        Ok(())
    }

    fn player_collected_item(&mut self, player_id: PlayerId, item: Item) -> LobbyResult<()> {
        let player = self
            .shared
            .players
            .get_mut(&player_id)
            .ok_or(LobbyError::PlayerInvalid(player_id))?;

        match item {
            Item::Spatula(spat) => {
                let state = self
                    .shared
                    .game_state
                    .spatulas
                    .entry(spat)
                    .or_insert_with(SpatulaState::default);

                // This can happen in rare situations where the player colllected an exhausted spatula
                // before receiving the lobby update that exhausted it. We should just ignore this case
                if state.collection_count == self.shared.options.tier_count {
                    log::info!(
                        "Player {player_id:#X} tried to collect exhausted spatula {spat:?}.",
                    );
                    return Ok(());
                }

                if state.collection_vec.contains(&player_id) {
                    return Err(LobbyError::InvalidAction(player_id));
                }

                player.score += GAME_CONSTS.spat_scores[state.collection_count as usize];

                state
                    .collection_vec
                    .insert(state.collection_count as usize, player_id);
                log::info!(
                    "Player {player_id:#X} collected {spat:?} with tier {:?}",
                    state.collection_count
                );

                state.collection_count += 1;

                if spat == Spatula::TheSmallShallRuleOrNot {
                    self.stop_game();
                }

                let message = Message::Lobby(LobbyMessage::GameLobbyInfo {
                    lobby: self.shared.clone(),
                });
                let _ = self.sender.send(message);
            }
        }
        Ok(())
    }

    fn set_game_options(&mut self, player_id: PlayerId, options: LobbyOptions) -> LobbyResult<()> {
        if self.shared.host_id != Some(player_id) {
            return Err(LobbyError::NeedsHost);
        }
        self.shared.options = options;

        let message = Message::Lobby(LobbyMessage::GameLobbyInfo {
            lobby: self.shared.clone(),
        });
        let _ = self.sender.send(message);
        Ok(())
    }
}

// TODO: Test that correct messages are broadcast once protocol is updated to send incremental events
#[cfg(test)]
mod test {
    use std::time::Duration;

    use bfbb::{Level, Spatula};
    use clash_lib::{lobby::GamePhase, net::Item, player::PlayerOptions};
    use tokio::{sync::mpsc, time::timeout};

    use crate::lobby::{lobby_handle::LobbyHandleProvider, LobbyError};

    use super::LobbyActor;

    fn setup() -> LobbyActor {
        let (_, rx) = mpsc::channel(2);
        LobbyActor::new(Default::default(), rx, 0)
    }

    #[test]
    fn start_game() {
        let mut lobby = setup();
        lobby.add_player(0).unwrap();
        lobby.add_player(1).unwrap();

        // Only the host can start a game
        assert_eq!(lobby.start_game(1), Err(LobbyError::NeedsHost));

        // Starting while not all players are on the main menu silently fails (at least for now)
        lobby.set_player_can_start(0, true).unwrap();
        assert!(lobby.start_game(0).is_ok());
        assert_eq!(lobby.shared.game_phase, GamePhase::Setup);

        // Now we can start
        lobby.set_player_can_start(1, true).unwrap();
        assert!(lobby.start_game(0).is_ok());
        assert_eq!(lobby.shared.game_phase, GamePhase::Playing);
    }

    #[test]
    fn add_player() {
        let mut lobby = setup();

        for i in 0..clash_lib::MAX_PLAYERS as u32 {
            assert!(lobby.add_player(i).is_ok());
            assert!(lobby.shared.players.contains_key(&i));
        }
        assert_eq!(lobby.shared.host_id, Some(0));

        // Adding a seventh player will fail
        assert!(matches!(lobby.add_player(6), Err(LobbyError::LobbyFull)));
    }

    #[test]
    fn remove_player() {
        let mut lobby = setup();
        lobby.add_player(0).unwrap();
        lobby.add_player(1).unwrap();

        // Removing the host will assign a new one
        lobby.rem_player(0);
        assert!(!lobby.shared.players.contains_key(&0));
        assert_eq!(lobby.shared.host_id, Some(1));

        // Removing the last player will close the lobby
        lobby.rem_player(1);
        assert!(lobby.shared.players.is_empty());
    }

    #[test]
    fn set_player_options() {
        let mut lobby = setup();
        lobby.add_player(0).unwrap();
        lobby.add_player(1).unwrap();

        // Can't set options for a non-existant player
        assert_eq!(
            lobby.set_player_options(1337, Default::default()),
            Err(LobbyError::PlayerInvalid(1337))
        );

        let square = PlayerOptions {
            name: "Parallelogram".to_owned(),
            ..Default::default()
        };
        let rectangle = PlayerOptions {
            name: "Rectangle".to_owned(),
            ..Default::default()
        };
        assert!(lobby.set_player_options(0, square).is_ok());
        assert!(lobby.set_player_options(1, rectangle).is_ok());

        assert_eq!(
            lobby.shared.players.get(&0).unwrap().options.name,
            "Parallelogram"
        );
        assert_eq!(
            lobby.shared.players.get(&1).unwrap().options.name,
            "Rectangle"
        );
    }

    #[test]
    fn set_player_level() {
        let mut lobby = setup();
        lobby.add_player(0).unwrap();
        lobby.add_player(1).unwrap();

        // Can't set level for a non-existant player
        assert_eq!(
            lobby.set_player_level(1337, Default::default()),
            Err(LobbyError::PlayerInvalid(1337))
        );

        assert!(lobby.set_player_level(0, Some(Level::BikiniBottom)).is_ok());
        assert!(lobby.set_player_level(1, Some(Level::ShadyShoals)).is_ok());

        assert_eq!(
            lobby.shared.players.get(&0).unwrap().current_level,
            Some(Level::BikiniBottom)
        );
        assert_eq!(
            lobby.shared.players.get(&1).unwrap().current_level,
            Some(Level::ShadyShoals)
        );
    }

    #[test]
    fn collect_small_shall_rule() {
        let mut lobby = setup();
        lobby.add_player(0).unwrap();

        // Collecting Small Shall Rule finishes the match
        assert!(lobby
            .player_collected_item(0, Item::Spatula(Spatula::TheSmallShallRuleOrNot))
            .is_ok());
        // TODO: This will transition to GamePhase::Finished when the GUI is implemented for it
        assert_eq!(lobby.shared.game_phase, GamePhase::Setup);
    }

    #[test]
    fn player_collected_item_state() {
        let mut lobby = setup();
        lobby.add_player(0).unwrap();
        lobby.add_player(1).unwrap();

        assert!(lobby
            .player_collected_item(0, Item::Spatula(Spatula::SpongebobsCloset))
            .is_ok());
        assert!(lobby
            .player_collected_item(1, Item::Spatula(Spatula::SpongebobsCloset))
            .is_ok());
        assert!(lobby
            .player_collected_item(1, Item::Spatula(Spatula::OnTopOfThePineapple))
            .is_ok());

        // Only two unique spatulas were collected
        assert_eq!(lobby.shared.game_state.spatulas.len(), 2);
        assert!(lobby
            .shared
            .game_state
            .spatulas
            .contains_key(&Spatula::SpongebobsCloset));
        assert!(lobby
            .shared
            .game_state
            .spatulas
            .contains_key(&Spatula::OnTopOfThePineapple));
    }

    #[test]
    fn player_collected_item_score() {
        let mut lobby = setup();
        lobby.add_player(0).unwrap();
        lobby.add_player(1).unwrap();
        lobby.add_player(2).unwrap();
        lobby.add_player(3).unwrap();

        // Non-existant player can't collect an item
        assert_eq!(
            lobby.player_collected_item(1337, Item::Spatula(Spatula::CowaBungee)),
            Err(LobbyError::PlayerInvalid(1337))
        );

        // Collecting a spatula first grants highest score
        assert!(lobby
            .player_collected_item(0, Item::Spatula(Spatula::SpongebobsCloset))
            .is_ok());
        assert!(lobby
            .player_collected_item(1, Item::Spatula(Spatula::OnTopOfThePineapple))
            .is_ok());
        assert!(lobby
            .player_collected_item(0, Item::Spatula(Spatula::CowaBungee))
            .is_ok());

        // A new player collecting a spatula again gives fewer points
        assert!(lobby
            .player_collected_item(1, Item::Spatula(Spatula::SpongebobsCloset))
            .is_ok());
        assert!(lobby
            .player_collected_item(2, Item::Spatula(Spatula::SpongebobsCloset))
            .is_ok());

        let points = &clash_lib::GAME_CONSTS.spat_scores;
        assert_eq!(lobby.shared.players.get(&0).unwrap().score, points[0] * 2);
        assert_eq!(
            lobby.shared.players.get(&1).unwrap().score,
            points[0] + points[1]
        );
        assert_eq!(lobby.shared.players.get(&2).unwrap().score, points[2]);
        assert_eq!(lobby.shared.players.get(&3).unwrap().score, 0);
    }

    #[test]
    fn player_collected_item_max() {
        let mut lobby = setup();
        lobby.add_player(0).unwrap();
        lobby.add_player(1).unwrap();
        lobby.add_player(2).unwrap();
        lobby.add_player(3).unwrap();

        for i in 0..=2 {
            assert!(lobby
                .player_collected_item(i, Item::Spatula(Spatula::SpongebobsCloset))
                .is_ok());
        }

        // A new player collecting an exhausted spatula will simply be ignored
        assert_eq!(
            lobby.player_collected_item(3, Item::Spatula(Spatula::SpongebobsCloset)),
            Ok(())
        );
        let closet_state = lobby
            .shared
            .game_state
            .spatulas
            .get(&Spatula::SpongebobsCloset)
            .unwrap();
        assert!(!closet_state.collection_vec.contains(&3),);
        assert_eq!(lobby.shared.players.get(&3).unwrap().score, 0);

        lobby.shared.options.tier_count = 1;
        assert!(lobby
            .player_collected_item(0, Item::Spatula(Spatula::OnTopOfThePineapple))
            .is_ok());
        assert_eq!(
            lobby.player_collected_item(1, Item::Spatula(Spatula::OnTopOfThePineapple)),
            Ok(())
        );
    }

    #[test]
    fn player_collected_item_twice() {
        let mut lobby = setup();
        lobby.add_player(0).unwrap();

        // Same player can't collect an item that they already collected once
        assert!(lobby
            .player_collected_item(0, Item::Spatula(Spatula::CowaBungee))
            .is_ok());
        assert_eq!(
            lobby.player_collected_item(0, Item::Spatula(Spatula::CowaBungee)),
            Err(LobbyError::InvalidAction(0))
        );
        assert_eq!(
            lobby
                .shared
                .game_state
                .spatulas
                .get(&Spatula::CowaBungee)
                .unwrap()
                .collection_count,
            1
        );

        let first_points = clash_lib::GAME_CONSTS.spat_scores[0];
        assert_eq!(lobby.shared.players.get(&0).unwrap().score, first_points);
    }

    #[tokio::test]
    async fn lobby_dies() {
        let get_lobby = || {
            let (tx, rx) = mpsc::channel(2);
            let mut actor = LobbyActor::new(Default::default(), rx, 0);
            let handle = LobbyHandleProvider {
                sender: tx,
                lobby_id: 0,
            }
            .get_handle(0);
            actor.add_player(0).unwrap();
            (actor, handle)
        };

        // The lobby will run for as long as handles remain
        {
            let (actor, handle) = get_lobby();
            timeout(Duration::from_secs(1), actor.run())
                .await
                .expect_err("Lobby closed with handles still remaining");
            // Explicitly drop handle to ensure it's not dropped early
            drop(handle)
        }

        // The lobby will die when the last handle (Sender) is dropped
        {
            let (actor, handle) = get_lobby();

            drop(handle);
            timeout(Duration::from_secs(1), actor.run())
                .await
                .expect("Lobby failed to close");
        }

        // Alternatively, the lobby will die when the last player is removed
        {
            let (mut actor, handle) = get_lobby();

            actor.rem_player(0);
            timeout(Duration::from_secs(1), actor.run())
                .await
                .expect("Lobby failed to close");
            // Explicitly drop handle to ensure it's not dropped early
            assert_eq!(handle.start_game().await, Err(LobbyError::HandleInvalid));
        }
    }
}
