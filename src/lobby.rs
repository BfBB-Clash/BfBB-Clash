use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Options {
    pub max_spats: u8,
    pub ng_plus: bool,
}

pub struct SharedLobby {
    pub lobby_id: u32,
    pub options: Options,
    pub is_started: bool,
    pub host_index: u8,
    pub player_count: u8, //Probably will never be larger than a u8 :)
}

pub trait LobbyTrait {
    fn lobby_start(&mut self);
    fn lobby_stop(&mut self);
}