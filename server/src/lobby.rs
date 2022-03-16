use clash::lobby::{LobbyOptions, SharedLobby};
use clash::player::SharedPlayer;
use clash::protocol::Message;
use clash::{AuthId, LobbyId, MAX_PLAYERS};
use thiserror::Error;
use tokio::sync::broadcast::Sender;
use tokio::sync::broadcast::{channel, Receiver};

use crate::PlayerMap;

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
    pub fn new(new_options: LobbyOptions, lobby_id: LobbyId, host_id: AuthId) -> Self {
        let (sender, _) = channel(100);
        Self {
            shared: SharedLobby::new(lobby_id, new_options, host_id),
            sender,
        }
    }

    pub fn is_player_in_lobby(&mut self, auth_id: &u32) -> bool {
        self.shared.players.contains_key(auth_id)
    }

    pub fn start_game() {}

    pub fn rem_player(&mut self, players: &mut PlayerMap, auth_id: &AuthId) {
        self.shared.players.remove(auth_id);
        players.remove(auth_id);
    }

    pub fn add_player(
        &mut self,
        players: &mut PlayerMap,
        auth_id: AuthId,
    ) -> Result<(Sender<Message>, Receiver<Message>), LobbyError> {
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

        let player = SharedPlayer::default();
        self.shared.players.insert(auth_id, player);
        Ok((self.sender.clone(), self.sender.subscribe()))
    }
}
