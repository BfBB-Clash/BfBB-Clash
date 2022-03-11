use clash::player::SharedPlayer;

#[derive(Debug)]
pub struct Player {
    pub shared: SharedPlayer,
    pub auth_id: u32,
}

impl Player {
    pub fn new(shared: SharedPlayer, auth_id: u32) -> Self {
        Self { shared, auth_id }
    }
}
