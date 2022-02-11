
pub struct PlayerOptions {
    pub name: String,
    pub color: u32, //TODO: Implement this.
    //Other options?
}

pub struct Player {
    options: PlayerOptions,
}

impl Player {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}
