use std::collections::HashMap;

use bfbb::{EnumCount, Spatula};
use serde::{Deserialize, Serialize};

use crate::PlayerId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatulaState {
    pub collection_count: u8,
    pub collection_vec: Vec<PlayerId>,
}

impl Default for SpatulaState {
    fn default() -> Self {
        Self {
            collection_count: 0,
            collection_vec: Vec::new(),
        }
    }
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
