use clash::player::{SharedPlayer, PlayerOptions};

pub struct Player {
    shared: SharedPlayer,
    auth_id: u32,
    lobby_id: u32,

}

impl Player {
    pub fn new(shared: SharedPlayer, auth_id: u32) -> Self
    {
        Self {
            shared,
            auth_id,
            lobby_id: 0,
        }
    }
}