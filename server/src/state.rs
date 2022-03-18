use clash::{AuthId, LobbyId};
use rand::{thread_rng, Rng};
use std::collections::HashMap;

use crate::lobby::Lobby;

pub type PlayerMap = HashMap<AuthId, Option<LobbyId>>;
pub type LobbyMap = HashMap<LobbyId, Lobby>;

pub struct State {
    pub players: PlayerMap,
    pub lobbies: LobbyMap,
}

impl State {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
            lobbies: HashMap::new(),
        }
    }

    pub fn add_player(&mut self) -> u32 {
        let auth_id = self.gen_auth_id();
        self.players.insert(auth_id, None);
        auth_id
    }

    // TODO: dedupe this.
    fn gen_auth_id(&self) -> AuthId {
        let mut auth_id;
        loop {
            auth_id = thread_rng().gen();
            if !self.players.contains_key(&auth_id) {
                break;
            };
        }
        auth_id
    }

    pub fn gen_lobby_id(&self) -> LobbyId {
        let mut lobby_id;
        loop {
            lobby_id = thread_rng().gen();
            if !self.lobbies.contains_key(&lobby_id) {
                break;
            };
        }
        lobby_id
    }
}
