use serde::{Deserialize, Serialize};
use crate::player::SharedPlayer;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LobbyOptions {
    pub max_spats: u8,
    pub ng_plus: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SharedLobby {
    pub lobby_id: u32,
    pub options: LobbyOptions,
    pub players: Vec<SharedPlayer>,
    pub player_count: u8, //Probably will never be larger than a u8 :)
    pub is_started: bool,
    pub host_index: i8,
}

impl SharedLobby {
    pub fn new(lobby_id: u32, options: LobbyOptions, host_index: i8) -> Self {
        Self { lobby_id, options, players: Vec::new(), player_count: 1, is_started: false, host_index }
    }
}
