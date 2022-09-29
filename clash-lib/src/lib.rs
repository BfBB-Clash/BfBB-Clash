pub mod game_state;
pub mod lobby;
pub mod net;
pub mod player;

pub const MAX_PLAYERS: usize = 6;

pub struct GameRuleConsts {
    pub spat_scores: [u32; MAX_PLAYERS],
}

pub const GAME_CONSTS: GameRuleConsts = GameRuleConsts {
    spat_scores: [300, 100, 50, 40, 25, 15],
};

// NOTE: We can considering using the newtype pattern here to avoid the possiblity of mixing up these id types,
//       but it adds a lot of boilerplate and I'm not sure that it's actually worth it at this point.
pub type PlayerId = u32;
pub type LobbyId = u32;
