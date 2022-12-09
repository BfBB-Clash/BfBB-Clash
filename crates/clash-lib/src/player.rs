use ecolor::Color32;
use serde::{Deserialize, Serialize};

use bfbb::Level;

pub const COLORS: [(u8, u8, u8); 6] = [
    (195, 247, 58),
    (224, 108, 0),
    (89, 155, 108),
    (16, 130, 168),
    (176, 142, 184),
    (254, 154, 95),
];

#[derive(Default, Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct PlayerOptions {
    pub name: String,
    pub color: (u8, u8, u8),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NetworkedPlayer {
    pub options: PlayerOptions,
    pub current_level: Option<Level>,
    pub score: u32,
    pub menu_order: u8,
    pub ready_to_start: bool,
}

impl NetworkedPlayer {
    pub fn new(options: PlayerOptions, menu_order: u8) -> Self {
        Self {
            options,
            current_level: None,
            score: 0,
            menu_order,
            ready_to_start: false,
        }
    }

    pub fn reset(&mut self) {
        self.score = 0;
    }
}

impl PlayerOptions {
    pub fn color(&self) -> Color32 {
        Color32::from_rgb(self.color.0, self.color.1, self.color.2)
    }
}
