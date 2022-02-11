use clash::player::{SharedPlayer, PlayerOptions};

pub struct Player {
    shared: SharedPlayer,
    auth_id: u32,
    lobby_id: u32,

}