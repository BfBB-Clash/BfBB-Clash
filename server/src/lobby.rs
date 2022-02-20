use clash::lobby::{LobbyOptions, SharedLobby};
use clash::player::SharedPlayer;
use clash::protocol::Message;
use clash::MAX_PLAYERS;

use log::{debug, error, info, warn};
use std::iter::Iterator;
use std::sync::{self, mpsc::Sender, Arc, Mutex};

use crate::player::Player;
use crate::State;

pub struct Lobby {
    pub shared: SharedLobby,
    pub player_ids: Vec<u32>,
    pub host_id: u32,
}

impl Lobby {
    pub fn new(new_options: LobbyOptions, host_id: u32) -> Self {
        Self {
            shared: SharedLobby {
                lobby_id: 1,
                options: new_options,
                is_started: false,
                players: Vec::new(),
                host_index: 0,
                player_count: 0,
            },
            host_id: host_id,
            player_ids: vec![0; MAX_PLAYERS],
        }
    }

    pub fn add_player(&mut self, state: &mut State, auth_id: u32) -> Result<(), ()> {
        if self.shared.player_count < 6 {
            if let p = state.players.get_mut(&auth_id).unwrap() {
                self.shared.player_count += 1;
                self.shared.players.push(p.shared.clone());
                self.player_ids.push(auth_id);
                p.shared.lobby_index =
                    i8::try_from(self.player_ids.len()).expect("This shouldn't break.");
                return Ok(());
            }
            return Err(());
        }
        Err(())
    }

    pub fn broadcast_message(&self, state: &mut State, message: Message) -> Result<(), ()> {
        for &auth_id in self.player_ids.iter() {
            if auth_id != 0 {
                match state.players.get_mut(&auth_id) {
                    Some(p) => {
                        // TODO: remove unwrap/ok.
                        p.send.lock().unwrap().send(message.clone()).ok();
                    }
                    None => {
                        //TODO: Freak out more probably.
                        info!("Unknown player id {auth_id:#X}");
                        continue;
                    }
                };
            }
        }
        Ok(())
    }
}
