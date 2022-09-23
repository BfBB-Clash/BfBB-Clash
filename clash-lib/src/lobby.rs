use std::collections::HashMap;

use crate::{game_state::GameState, player::NetworkedPlayer, LobbyId, PlayerId};
use bfbb::Level;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct LobbyOptions {
    pub lab_door_cost: u8,
    pub ng_plus: bool,
}

impl Default for LobbyOptions {
    fn default() -> Self {
        Self {
            lab_door_cost: 75,
            ng_plus: false,
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GamePhase {
    Setup,
    Playing,
    Finished,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NetworkedLobby {
    pub game_state: GameState,
    pub lobby_id: LobbyId,
    pub options: LobbyOptions,
    pub players: HashMap<PlayerId, NetworkedPlayer>,
    pub game_phase: GamePhase,
    pub host_id: Option<PlayerId>,
}

impl NetworkedLobby {
    pub fn new(lobby_id: u32) -> Self {
        Self {
            game_state: GameState::default(),
            lobby_id,
            options: LobbyOptions::default(),
            players: HashMap::new(),
            game_phase: GamePhase::Setup,
            host_id: None,
        }
    }

    /// True when all connected players are on the Main Menu
    pub fn can_start(&self) -> bool {
        // TODO: Solve the "Demo Cutscene" issue. We can probably detect when players are on the autosave preference screen instead.
        self.players
            .values()
            .all(|p| p.current_level == Some(Level::MainMenu))
    }
}

#[cfg(test)]
mod tests {
    use super::NetworkedLobby;
    use crate::player::{NetworkedPlayer, PlayerOptions};

    use bfbb::Level;

    #[test]
    fn can_start() {
        let mut lobby = NetworkedLobby::new(0);
        let player_0 = lobby
            .players
            .entry(0)
            .or_insert_with(|| NetworkedPlayer::new(PlayerOptions::default(), 0));
        player_0.current_level = Some(Level::MainMenu);

        assert!(lobby.can_start());

        lobby
            .players
            .entry(1)
            .or_insert_with(|| NetworkedPlayer::new(PlayerOptions::default(), 1));
        assert!(!lobby.can_start());

        lobby.players.get_mut(&1).unwrap().current_level = Some(Level::MainMenu);
        assert!(lobby.can_start());
    }
}
