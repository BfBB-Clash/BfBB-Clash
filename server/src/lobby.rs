use clash::lobby::{LobbyOptions, SharedLobby};
use clash::player::SharedPlayer;
use clash::protocol::Message;
use clash::{LobbyId, PlayerId, MAX_PLAYERS};
use thiserror::Error;
use tokio::sync::broadcast::Sender;
use tokio::sync::broadcast::{channel, Receiver};

use crate::state::PlayerMap;

#[derive(Copy, Clone, Debug, Error)]
pub enum LobbyError {
    #[error("Attempted to add a player to a full lobby")]
    LobbyFull,
    #[error("Attempted to add a invalid player to a lobby")]
    PlayerInvalid,
}

pub struct Lobby {
    pub shared: SharedLobby,
    pub sender: Sender<Message>,
}

impl Lobby {
    pub fn new(new_options: LobbyOptions, lobby_id: LobbyId) -> Self {
        let (sender, _) = channel(100);
        Self {
            shared: SharedLobby::new(lobby_id, new_options),
            sender,
        }
    }

    pub fn start_game(&mut self) {
        self.shared.is_started = true;
        if self.sender.send(Message::GameBegin {}).is_err() {
            log::warn!(
                "Lobby {:#X} started with no players in lobby.",
                self.shared.lobby_id
            )
        }
    }

    pub fn is_player_in_lobby(&mut self, player_id: &u32) -> bool {
        self.shared.players.contains_key(player_id)
    }

    /// Adds a new player to this lobby. If there is currently no host, they will become it.
    pub fn add_player(
        &mut self,
        players: &mut PlayerMap,
        player_id: PlayerId,
    ) -> Result<Receiver<Message>, LobbyError> {
        if self.shared.players.len() >= MAX_PLAYERS {
            return Err(LobbyError::LobbyFull);
        }

        if self.is_player_in_lobby(&player_id) {
            return Err(LobbyError::PlayerInvalid);
        }

        // Make sure the player isn't already in a different lobby
        if players
            .get(&player_id)
            .ok_or(LobbyError::PlayerInvalid)?
            .is_some()
        {
            return Err(LobbyError::PlayerInvalid);
        }
        players.insert(player_id, Some(self.shared.lobby_id));
        // TODO: Unhardcode player color
        let mut player = SharedPlayer::default();
        player.options.color = clash::player::COLORS[self.shared.players.len()];

        self.shared.players.insert(player_id, player);
        if self.shared.host_id == None {
            self.shared.host_id = Some(player_id);
        }

        Ok(self.sender.subscribe())
    }

    // Removes a player from the lobby, if it exists, returning the number of player's remaining
    pub fn rem_player(&mut self, player_id: PlayerId) -> usize {
        self.shared.players.remove(&player_id);
        if self.shared.host_id == Some(player_id) {
            // Pass host to first remaining player in list (effectively random with a HashMap)
            // NOTE: We could consider passing host based on join order
            self.shared.host_id = self.shared.players.iter().next().map(|(&id, _)| id);
        }

        // Update remaining clients of the change
        let _ = self.sender.send(Message::GameLobbyInfo {
            lobby: self.shared.clone(),
        });
        self.shared.players.len()
    }
}
