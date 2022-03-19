use std::collections::HashMap;

use crate::{game_state::GameState, player::SharedPlayer, LobbyId, PlayerId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LobbyOptions {
    pub lab_door_cost: u8,
    pub ng_plus: bool,
}

impl Default for LobbyOptions {
    fn default() -> Self {
        Self {
            lab_door_cost: 75,
            ng_plus: false,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SharedLobby {
    pub game_state: GameState,
    pub lobby_id: LobbyId,
    pub options: LobbyOptions,
    pub players: HashMap<PlayerId, SharedPlayer>,
    pub is_started: bool,
    pub host_id: Option<PlayerId>,
}

impl SharedLobby {
    pub fn new(lobby_id: u32, options: LobbyOptions) -> Self {
        Self {
            game_state: GameState::default(),
            lobby_id,
            options,
            players: HashMap::new(),
            is_started: false,
            host_id: None,
        }
    }
}
