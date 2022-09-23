use std::collections::HashMap;

use bfbb::{EnumCount, Spatula};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumCount, EnumIter};

use crate::PlayerId;

// TODO: Maybe don't hardcode these?
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, EnumCount, Serialize, Deserialize)]
pub enum SpatulaTier {
    Golden = 0,
    Silver,
    Bronze,
    None,
}

impl From<i32> for SpatulaTier {
    fn from(v: i32) -> Self {
        match v {
            x if x == Self::Golden as i32 => Self::Golden,
            x if x == Self::Silver as i32 => Self::Silver,
            x if x == Self::Bronze as i32 => Self::Bronze,
            _ => Self::None,
        }
    }
}

impl SpatulaTier {
    pub fn get_color(&mut self) -> (u8, u8, u8) {
        // Taken from google :)
        match self {
            SpatulaTier::Golden => (0xd4, 0xaf, 0x37),
            SpatulaTier::Silver => (0xc0, 0xc0, 0xc0),
            SpatulaTier::Bronze => (0xcd, 0x7f, 0x32),
            _ => (0, 0, 0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatulaState {
    pub tier: SpatulaTier,
    pub collection_vec: Vec<PlayerId>,
}

impl Default for SpatulaState {
    fn default() -> Self {
        Self {
            tier: SpatulaTier::Golden,
            collection_vec: Vec::with_capacity(SpatulaTier::COUNT),
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
