use std::sync::{self, Arc, Mutex, RwLock, mpsc::Sender};

use clash::player::{PlayerOptions, SharedPlayer};
use clash::protocol::Message;

#[derive(Debug)]
pub struct Player {
    pub shared: SharedPlayer,
    pub auth_id: u32,
    pub lobby_id: u32,
    pub send: Mutex<Sender<Message>>,
}

impl Player {
    pub fn new(shared: SharedPlayer, auth_id: u32, send: Mutex<Sender<Message>>) -> Self {
        Self {
            shared,
            auth_id,
            lobby_id: 0,
            send
        }
    }
}
