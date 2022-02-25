use std::collections::HashMap;

use clash::spatula::Spatula;

pub struct GameState {
    pub spatulas: HashMap<Spatula, Option<usize>>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            spatulas: HashMap::with_capacity(Spatula::COUNT),
        }
    }
}