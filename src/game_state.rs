use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use strum::EnumCount;

use crate::{room::Room, spatula::Spatula};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub spatulas: HashMap<Spatula, Option<usize>>,
    // TODO: needs to be per-player
    pub current_room: Option<Room>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            spatulas: HashMap::with_capacity(Spatula::COUNT),
            current_room: None,
        }
    }
}
