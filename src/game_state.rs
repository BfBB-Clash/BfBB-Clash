use std::collections::HashMap;

use strum::EnumCount;

use crate::{lobby::SharedLobby, room::Room, spatula::Spatula};

pub struct GameState {
    pub lobby: SharedLobby,
    pub spatulas: HashMap<Spatula, Option<usize>>,
    pub current_room: Option<Room>,
}

impl GameState {
    pub fn new(lobby: SharedLobby) -> Self {
        Self {
            lobby,
            spatulas: HashMap::with_capacity(Spatula::COUNT),
            current_room: None,
        }
    }
}
