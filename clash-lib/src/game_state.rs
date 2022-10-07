use std::collections::HashMap;

use bfbb::{EnumCount, Spatula};
use serde::{Deserialize, Serialize};

use crate::PlayerId;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SpatulaState {
    pub collection_vec: Vec<PlayerId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// Mapping from between spatulas and how many times it's been collected.
    pub spatulas: HashMap<Spatula, SpatulaState>,
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
