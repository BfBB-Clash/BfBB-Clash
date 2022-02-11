use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct LobbyOptions {
    pub max_spats: u8,
    pub ng_plus: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SharedLobby {
    pub lobby_id: u32,
    pub options: LobbyOptions,
    pub is_started: bool,
    pub host_index: u8,
    pub player_count: u8, //Probably will never be larger than a u8 :)
}

impl SharedLobby {
    pub fn new(lobby_id: u32, options: LobbyOptions, is_started: bool, host_index: u8, player_count: u8) -> Self {
        Self { lobby_id, options, is_started, host_index, player_count }
    }
}

pub trait LobbyTrait {
    fn lobby_start(&mut self);
    fn lobby_stop(&mut self);
}