
#[derive(Debug)]
pub struct Options {
    pub max_spats: u8,
    pub ng_plus: bool,
}

pub struct SharedLobby {
    lobby_id: u32,
    options: Options,
    is_started: bool,
    host_index: u8,
    player_count: u8, //Probably will never be larger than a u8 :)
}

pub trait LobbyTrait {
    fn lobby_start(&mut self);
    fn lobby_stop(&mut self);
}