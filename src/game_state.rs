use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use strum::EnumCount;

use crate::{spatula::Spatula, AuthId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub spatulas: HashMap<Spatula, Option<AuthId>>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            spatulas: HashMap::with_capacity(Spatula::COUNT),
        }
    }
}
