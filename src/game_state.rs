use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::PlayerId;
use bfbb::{EnumCount, Spatula};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub spatulas: HashMap<Spatula, Option<PlayerId>>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            spatulas: HashMap::with_capacity(Spatula::COUNT),
        }
    }
}

impl GameState {
    pub fn reset_state(&mut self) {
        self.spatulas.clear();
    }
}