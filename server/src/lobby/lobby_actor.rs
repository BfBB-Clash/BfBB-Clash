use std::collections::hash_map::Entry;

use bfbb::{Level, Spatula};
use clash::lobby::{GamePhase, LobbyOptions, NetworkedLobby};
use clash::net::{Item, Message};
use clash::player::{NetworkedPlayer, PlayerOptions};
use clash::{LobbyId, PlayerId, MAX_PLAYERS};
use tokio::sync::{broadcast, mpsc, oneshot};

use crate::state::ServerState;

use super::{LobbyError, LobbyResult};

pub struct LobbyActor {
    state: ServerState,
    receiver: mpsc::Receiver<LobbyMessage>,
    shared: NetworkedLobby,
    sender: broadcast::Sender<Message>,
    next_menu_order: u8,
}

#[derive(Debug)]
pub enum LobbyMessage {
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
        receiver: mpsc::Receiver<LobbyMessage>,
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
                LobbyMessage::StartGame { respond_to, id } => {
                    let _ = respond_to.send(self.start_game(id));
                }
                LobbyMessage::AddPlayer { respond_to, id } => {
                    let _ = respond_to.send(self.add_player(id));
                }
                LobbyMessage::RemovePlayer { id } => self.rem_player(id),
                LobbyMessage::SetPlayerOptions {
                    respond_to,
                    id,
                    options,
                } => {
                    let _ = respond_to.send(self.set_player_options(id, options));
                }
                LobbyMessage::SetPlayerLevel {
                    respond_to,
                    id,
                    level,
                } => {
                    let _ = respond_to.send(self.set_player_level(id, level));
                }
                LobbyMessage::PlayerCollectedItem {
                    respond_to,
                    id,
                    item,
                } => {
                    let _ = respond_to.send(self.player_collected_item(id, item));
                }
                LobbyMessage::SetGameOptions {
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

    fn start_game(&mut self, player_id: PlayerId) -> LobbyResult<()> {
        if self.shared.host_id != Some(player_id) {
            return Err(LobbyError::NeedsHost);
        }

        if !self.shared.can_start() {
            log::warn!(
                "Lobby {:#X} attempted to start when some players aren't on the Main Menu",
                self.shared.lobby_id
            );
            // Maybe this should be an error, I'm not sure
            return Ok(());
        }

        self.shared.game_phase = GamePhase::Playing;
        if self.sender.send(Message::GameBegin).is_err() {
            log::warn!(
                "Lobby {:#X} started with no players in lobby.",
                self.shared.lobby_id
            )
        }

        Ok(())
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
        player.options.color = clash::player::COLORS[self.shared.players.len()];
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

        let _ = self.sender.send(Message::GameLobbyInfo {
            lobby: self.shared.clone(),
        });

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
        let _ = self.sender.send(Message::GameLobbyInfo {
            lobby: self.shared.clone(),
        });

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

        let message = Message::GameLobbyInfo {
            lobby: self.shared.clone(),
        };
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

        let message = Message::GameLobbyInfo {
            lobby: self.shared.clone(),
        };
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
                if let Entry::Vacant(e) = self.shared.game_state.spatulas.entry(spat) {
                    e.insert(Some(player_id));
                    player.score += 1;
                    log::info!("Player {:#X} collected {spat:?}", player_id);

                    if spat == Spatula::TheSmallShallRuleOrNot {
                        self.shared.game_phase = GamePhase::Finished;
                    }

                    let message = Message::GameLobbyInfo {
                        lobby: self.shared.clone(),
                    };
                    let _ = self.sender.send(message);
                }
            }
        }
        Ok(())
    }

    fn set_game_options(&mut self, player_id: PlayerId, options: LobbyOptions) -> LobbyResult<()> {
        if self.shared.host_id != Some(player_id) {
            return Err(LobbyError::NeedsHost);
        }
        self.shared.options = options;

        let message = Message::GameLobbyInfo {
            lobby: self.shared.clone(),
        };
        let _ = self.sender.send(message);
        Ok(())
    }
}

// TODO: Test that correct messages are broadcast once protocol is updated to send incremental events
#[cfg(test)]
mod test {
    use std::time::Duration;

    use bfbb::{Level, Spatula};
    use clash::{lobby::GamePhase, net::Item, player::PlayerOptions};
    use tokio::{sync::mpsc, time::timeout};

    use crate::lobby::{lobby_handle::LobbyHandle, LobbyError};

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
        lobby.set_player_level(0, Some(Level::MainMenu)).unwrap();
        assert!(lobby.start_game(0).is_ok());
        assert_eq!(lobby.shared.game_phase, GamePhase::Setup);

        // Now we can start
        lobby.set_player_level(1, Some(Level::MainMenu)).unwrap();
        assert!(lobby.start_game(0).is_ok());
        assert_eq!(lobby.shared.game_phase, GamePhase::Playing);
    }

    #[test]
    fn add_player() {
        let mut lobby = setup();

        for i in 0..clash::MAX_PLAYERS as u32 {
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
    fn player_collected_item() {
        let mut lobby = setup();
        lobby.add_player(0).unwrap();
        lobby.add_player(1).unwrap();

        // Non-existant player can't collect an item
        assert_eq!(
            lobby.player_collected_item(1337, Item::Spatula(Spatula::CowaBungee)),
            Err(LobbyError::PlayerInvalid(1337))
        );

        // Collecting a spatula increases score by 1
        assert!(lobby
            .player_collected_item(0, Item::Spatula(Spatula::SpongebobsCloset))
            .is_ok());
        assert!(lobby
            .player_collected_item(1, Item::Spatula(Spatula::OnTopOfThePineapple))
            .is_ok());
        assert!(lobby
            .player_collected_item(0, Item::Spatula(Spatula::CowaBungee))
            .is_ok());

        assert_eq!(
            lobby.shared.game_state.spatulas,
            [
                (Spatula::SpongebobsCloset, Some(0)),
                (Spatula::OnTopOfThePineapple, Some(1)),
                (Spatula::CowaBungee, Some(0)),
            ]
            .into()
        );
        assert_eq!(lobby.shared.players.get(&0).unwrap().score, 2);
        assert_eq!(lobby.shared.players.get(&1).unwrap().score, 1);

        // Can't collect an item that has already been collected
        // This is expected behavior though, so it should fail silently
        assert!(lobby
            .player_collected_item(1, Item::Spatula(Spatula::CowaBungee))
            .is_ok());
        assert_eq!(
            lobby
                .shared
                .game_state
                .spatulas
                .get(&Spatula::CowaBungee)
                .unwrap(),
            &Some(0)
        );
        assert_eq!(lobby.shared.players.get(&1).unwrap().score, 1);

        // Collecting Small Shall Rule finishes the match
        assert!(lobby
            .player_collected_item(0, Item::Spatula(Spatula::TheSmallShallRuleOrNot))
            .is_ok());
        assert_eq!(lobby.shared.game_phase, GamePhase::Finished);
    }

    #[tokio::test]
    async fn lobby_dies() {
        let get_lobby = || {
            let (tx, rx) = mpsc::channel(2);
            let mut actor = LobbyActor::new(Default::default(), rx, 0);
            let handle = LobbyHandle {
                sender: tx,
                lobby_id: 0,
            };
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
            assert_eq!(handle.start_game(1).await, Err(LobbyError::HandleInvalid));
        }
    }
}
