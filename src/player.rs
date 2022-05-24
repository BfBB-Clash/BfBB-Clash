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
    pub menu_order: u8,
}

impl NetworkedPlayer {
    pub fn new(options: PlayerOptions, menu_order: u8) -> Self {
        Self {
            options,
            current_level: None,
            menu_order,
        }
    }
}

impl PlayerOptions {
    pub fn color(&self) -> epaint::Color32 {
        epaint::Color32::from_rgb(self.color.0, self.color.1, self.color.2)
    }
}
