use crate::{game_state::GameState, player::SharedPlayer};
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
    pub lobby_id: u32,
    pub options: LobbyOptions,
    pub players: Vec<SharedPlayer>,
    pub player_count: u32, //Probably will never be larger than a u8 :)
    pub is_started: bool,
    pub host_index: Option<usize>,
}

impl SharedLobby {
    pub fn new(lobby_id: u32, options: LobbyOptions, host_index: Option<usize>) -> Self {
        Self {
            game_state: GameState::default(),
            lobby_id,
            options,
            players: Vec::new(),
            player_count: 1,
            is_started: false,
            host_index,
        }
    }
}
