use std::collections::HashMap;

use crate::{game_state::GameState, player::NetworkedPlayer, LobbyId, PlayerId, MAX_PLAYERS};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct LobbyOptions {
    pub ng_plus: bool,
    pub lab_door_cost: u8,
    pub tier_count: u8,
    pub spat_scores: [u32; MAX_PLAYERS],
}

impl Default for LobbyOptions {
    fn default() -> Self {
        Self {
            lab_door_cost: 75,
            ng_plus: false,
            tier_count: 3,
            spat_scores: [100, 75, 50, 30, 20, 10],
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
    // TODO: Refactor this option out, we don't create a lobby until a player has connected to the server
    //       so we should be able to specify them as the host. When the last player leaves we close the lobby.
    pub host_id: Option<PlayerId>,
}

impl NetworkedLobby {
    pub fn new(lobby_id: impl Into<LobbyId>) -> Self {
        Self {
            game_state: GameState::default(),
            lobby_id: lobby_id.into(),
            options: LobbyOptions::default(),
            players: HashMap::new(),
            game_phase: GamePhase::Setup,
            host_id: None,
        }
    }

    pub fn reset(&mut self) {
        self.game_state.reset();
        self.game_phase = GamePhase::Setup;
        self.players.values_mut().for_each(NetworkedPlayer::reset);
    }

    /// True when all connected players are on the Main Menu
    pub fn can_start(&self) -> bool {
        // TODO: Now find a way to skip/remove the demo cutscene to make it easier to start a game
        self.players.values().all(|p| p.ready_to_start)
    }
}

#[cfg(test)]
mod tests {
    use bfbb::Spatula;

    use super::{GamePhase, NetworkedLobby};
    use crate::{
        game_state::SpatulaState,
        player::{NetworkedPlayer, PlayerOptions},
    };

    #[test]
    fn can_start() {
        let mut lobby = NetworkedLobby::new(0);
        let player_0 = lobby
            .players
            .entry(0.into())
            .or_insert_with(|| NetworkedPlayer::new(PlayerOptions::default(), 0));
        player_0.ready_to_start = true;

        assert!(lobby.can_start());

        lobby
            .players
            .entry(1.into())
            .or_insert_with(|| NetworkedPlayer::new(PlayerOptions::default(), 1));
        assert!(!lobby.can_start());

        lobby.players.get_mut(&1).unwrap().ready_to_start = true;
        assert!(lobby.can_start());
    }

    #[test]
    fn reset() {
        let mut lobby = NetworkedLobby::new(0);
        let player_0 = lobby
            .players
            .entry(0.into())
            .or_insert_with(|| NetworkedPlayer::new(PlayerOptions::default(), 0));

        lobby.game_phase = GamePhase::Playing;
        player_0.score = 100;
        lobby.game_state.spatulas.insert(
            Spatula::SpongebobsCloset,
            SpatulaState {
                collection_vec: vec![0.into()],
            },
        );

        lobby.reset();
        assert_eq!(lobby.game_phase, GamePhase::Setup);
        assert!(lobby.game_state.spatulas.is_empty());
        assert_eq!(lobby.players.len(), 1);
        assert_eq!(lobby.players.get(&0).unwrap().score, 0);
    }
}
