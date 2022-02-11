use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerOptions {
    pub name: String,
    pub color: u32, //TODO: Implement this.
    // Other options?
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SharedPlayer {
    options: PlayerOptions,
}

impl SharedPlayer {
    pub fn new(options: PlayerOptions) -> Self {
        Self { options }
    }
}
