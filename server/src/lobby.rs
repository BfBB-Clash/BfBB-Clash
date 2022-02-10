use clash::lobby::{SharedLobby, LobbyTrait, Options};

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
    fn new(options: Options) -> Self {
        Self{ options };
    }
}