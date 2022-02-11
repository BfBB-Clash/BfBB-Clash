use clash::lobby::{SharedLobby, LobbyTrait, LobbyOptions};

pub struct ServerLobby {
    shared: SharedLobby,
    players: Vec<u32>,
    host_id: u32,
}

impl LobbyTrait for ServerLobby {
    fn lobby_start(&mut self) {

    }
    fn lobby_stop(&mut self) {

    }
}

impl ServerLobby {
    fn new(new_options: LobbyOptions, host_id: u32) -> Self {
        Self { 
            shared: SharedLobby{ lobby_id: 1, options: new_options, is_started: false, host_index: 0, player_count: 1 },
            host_id: host_id,
            players: vec![0; 6],
        }
    }
}