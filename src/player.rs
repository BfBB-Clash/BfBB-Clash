use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlayerOptions {
    pub name: String,
    pub color: u32, //TODO: Implement this.
                    // Other options?
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SharedPlayer {
    pub options: PlayerOptions,
    pub current_lobby: u32,
    pub lobby_index: Option<usize>,
}

impl SharedPlayer {
    pub fn new(options: PlayerOptions) -> Self {
        Self {
            options,
            current_lobby: 0,
            lobby_index: None,
        }
    }
}
