use clash::lobby::{LobbyOptions, SharedLobby};
use clash::player::SharedPlayer;
use clash::protocol::Message;
use clash::{AuthId, LobbyId, MAX_PLAYERS};
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
        if self.sender.send(Message::GameBegin { auth_id: 0 }).is_err() {
            log::warn!(
                "Lobby {:#X} started with no players in lobby.",
                self.shared.lobby_id
            )
        }
    }

    pub fn is_player_in_lobby(&mut self, auth_id: &u32) -> bool {
        self.shared.players.contains_key(auth_id)
    }

    /// Adds a new player to this lobby. If there is currently no host, they will become it.
    pub fn add_player(
        &mut self,
        players: &mut PlayerMap,
        auth_id: AuthId,
    ) -> Result<Receiver<Message>, LobbyError> {
        if self.shared.players.len() >= MAX_PLAYERS {
            return Err(LobbyError::LobbyFull);
        }

        if self.is_player_in_lobby(&auth_id) {
            return Err(LobbyError::PlayerInvalid);
        }

        // Make sure the player isn't already in a different lobby
        if players
            .get(&auth_id)
            .ok_or(LobbyError::PlayerInvalid)?
            .is_some()
        {
            return Err(LobbyError::PlayerInvalid);
        }
        players.insert(auth_id, Some(self.shared.lobby_id));
        // TODO: Unhardcode player color
        let mut player = SharedPlayer::default();
        player.options.color = clash::player::COLORS[self.shared.players.len()];

        self.shared.players.insert(auth_id, player);
        if self.shared.host_id == None {
            self.shared.host_id = Some(auth_id);
        }

        Ok(self.sender.subscribe())
    }

    pub fn rem_player(&mut self, auth_id: AuthId) {
        // TODO: Remove this lobby if this is the last player (might need to be handled at callsite)
        self.shared.players.remove(&auth_id);
        if self.shared.host_id == Some(auth_id) {
            // Pass host to first remaining player in list (effectively random with a HashMap)
            // NOTE: We could consider passing host based on join order
            self.shared.host_id = self.shared.players.iter().next().map(|(&id, _)| id);
        }

        // Update remaining clients of the change
        let _ = self.sender.send(Message::GameLobbyInfo {
            auth_id: 0,
            lobby: self.shared.clone(),
        });
    }
}
