use clash::lobby::{LobbyOptions, SharedLobby};
use clash::protocol::Message;
use clash::MAX_PLAYERS;
use std::collections::HashMap;
use thiserror::Error;
use tokio::sync::broadcast::Sender;
use tokio::sync::broadcast::{channel, Receiver};

use crate::player::Player;

#[derive(Copy, Clone, Debug, Error)]
pub enum LobbyError {
    #[error("Attempted to add a player to a full lobby")]
    LobbyFull,
    #[error("Attempted to add a invalid player to a lobby")]
    PlayerInvalid,
}

pub struct Lobby {
    pub shared: SharedLobby,
    pub player_ids: Vec<u32>,
    pub host_id: u32,
    pub sender: Sender<Message>,
}

impl Lobby {
    pub fn new(new_options: LobbyOptions, host_id: u32) -> Self {
        let (sender, _) = channel(100);
        Self {
            shared: SharedLobby {
                lobby_id: 1,
                options: new_options,
                is_started: false,
                players: Vec::new(),
                host_index: None,
                player_count: 0,
            },
            host_id,
            player_ids: vec![0; MAX_PLAYERS as usize],
            sender,
        }
    }

    pub fn add_player(
        &mut self,
        players: &mut HashMap<u32, Player>,
        auth_id: u32,
    ) -> Result<(Sender<Message>, Receiver<Message>), LobbyError> {
        if self.shared.player_count < MAX_PLAYERS {
            // TODO: Check this.
            if let Some(p) = players.get_mut(&auth_id) {
                self.shared.player_count += 1;
                self.shared.players.push(p.shared.clone());
                self.player_ids.push(auth_id);
                p.shared.lobby_index = Some(self.player_ids.len());
                return Ok((self.sender.clone(), self.sender.subscribe()));
            }
            return Err(LobbyError::PlayerInvalid);
        }
        Err(LobbyError::LobbyFull)
    }
}
