use clash_lib::{LobbyId, PlayerId};
use hash_map::Entry;
use rand::{thread_rng, Rng};
use std::collections::{hash_map, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::lobby;
use crate::lobby::lobby_handle::LobbyHandleProvider;

pub type ServerState = Arc<Mutex<State>>;

#[derive(Debug, Default)]
pub struct State {
    pub players: HashSet<PlayerId>,
    pub lobbies: HashMap<LobbyId, LobbyHandleProvider>,
}

impl State {
    pub fn add_player(&mut self) -> PlayerId {
        let player_id = self.gen_player_id();
        self.players.insert(player_id);
        player_id
    }

    // TODO: Would be nice to not have to pass in a clone of ServerState here
    pub fn add_lobby(&mut self, state: ServerState) -> &mut LobbyHandleProvider {
        let lobby_id = self.gen_lobby_id();
        let handle = lobby::start_new_lobby(state, lobby_id);
        tracing::info!("Lobby {lobby_id:#X} opened");
        if let Entry::Vacant(e) = self.lobbies.entry(lobby_id) {
            return e.insert(handle);
        }
        unreachable!();
    }

    // TODO: dedupe this.
    fn gen_player_id(&self) -> PlayerId {
        let mut player_id;
        loop {
            player_id = thread_rng().gen();
            if !self.players.contains(&player_id) {
                break;
            };
        }
        player_id
    }

    fn gen_lobby_id(&self) -> LobbyId {
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
