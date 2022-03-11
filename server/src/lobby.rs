use clash::game_state::GameState;
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
    pub id: u32,
    pub shared: SharedLobby,
    pub player_ids: Vec<u32>,
    pub host_id: u32,
    pub sender: Sender<Message>,
}

impl Lobby {
    pub fn new(id: u32, new_options: LobbyOptions, lobby_id: u32, host_id: u32) -> Self {
        let (sender, _) = channel(100);
        Self {
            id,
            shared: SharedLobby {
                game_state: GameState::default(),
                lobby_id,
                options: new_options,
                is_started: false,
                players: Vec::with_capacity(MAX_PLAYERS),
                host_index: None,
            },
            host_id,
            player_ids: Vec::with_capacity(MAX_PLAYERS),
            sender,
        }
    }

    pub fn is_player_in_lobby(&mut self, auth_id: &u32) -> bool {
        let mut iter = self.player_ids.iter();
        for p in iter.next() {
            if *p == *auth_id {
                return true;
            }
        }
        false
    }

    pub fn start_game() {}

    pub fn rem_player(
        &mut self,
        players: &mut HashMap<u32, Player>,
        auth_id: u32,
    ) -> Result<(), LobbyError> {
        // TODO: Check this.
        if let Some(p) = players.get_mut(&auth_id) {
            if self.is_player_in_lobby(&auth_id) {
                self.shared.players.remove(
                    p.shared
                        .lobby_index
                        .expect("If the player is in the lobby, they should have a lobby_index."),
                );
                self.player_ids.remove(
                    p.shared
                        .lobby_index
                        .expect("If the player is in the lobby, they should have a lobby_index."),
                );
                return Ok(());
            }
        }
        return Err(LobbyError::PlayerInvalid);
    }

    pub fn add_player(
        &mut self,
        players: &mut HashMap<u32, Player>,
        auth_id: u32,
    ) -> Result<(Sender<Message>, Receiver<Message>), LobbyError> {
        if self.shared.players.len() < MAX_PLAYERS {
            // TODO: Check this.
            if let Some(p) = players.get_mut(&auth_id) {
                if !self.is_player_in_lobby(&auth_id) {
                    p.shared.lobby_index = Some(self.player_ids.len());
                    p.shared.current_lobby = self.id;
                    self.shared.players.push(p.shared.clone());
                    self.player_ids.push(auth_id);
                    return Ok((self.sender.clone(), self.sender.subscribe()));
                }
            }
            return Err(LobbyError::PlayerInvalid);
        }
        Err(LobbyError::LobbyFull)
    }
}
