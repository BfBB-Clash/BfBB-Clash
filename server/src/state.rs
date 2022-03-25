use anyhow::Context;
use clash::{lobby::LobbyOptions, protocol::ProtocolError, LobbyId, PlayerId};
use rand::{thread_rng, Rng};
use std::collections::HashMap;

use crate::lobby::Lobby;

pub type PlayerMap = HashMap<PlayerId, Option<LobbyId>>;
pub type LobbyMap = HashMap<LobbyId, Lobby>;

pub struct State {
    pub players: PlayerMap,
    pub lobbies: LobbyMap,
}

impl State {
    pub fn new() -> Self {
        Self {
            players: HashMap::new(),
            lobbies: HashMap::new(),
        }
    }

    pub fn add_player(&mut self) -> PlayerId {
        let player_id = self.gen_player_id();
        self.players.insert(player_id, None);
        player_id
    }

    pub fn add_lobby(&mut self) -> LobbyId {
        let gen_lobby_id = self.gen_lobby_id();
        self.lobbies.insert(
            gen_lobby_id,
            Lobby::new(LobbyOptions::default(), gen_lobby_id),
        );
        gen_lobby_id
    }

    pub fn get_lobby(&mut self, player_id: PlayerId) -> anyhow::Result<&mut Lobby> {
        let lobby_id = self
            .players
            .get(&player_id)
            .ok_or(ProtocolError::InvalidPlayerId(player_id))?
            .ok_or(ProtocolError::InvalidMessage)
            .context("Player not currently in a lobby")?;

        self.lobbies
            .get_mut(&lobby_id)
            .ok_or(ProtocolError::InvalidLobbyId(lobby_id))
            .context("Lobby specified by player list not found")
    }

    // TODO: dedupe this.
    fn gen_player_id(&self) -> PlayerId {
        let mut player_id;
        loop {
            player_id = thread_rng().gen();
            if !self.players.contains_key(&player_id) {
                break;
            };
        }
        player_id
    }

    fn gen_lobby_id(&self) -> LobbyId {
        let mut lobby_id;
        loop {
            lobby_id = thread_rng().gen();
            if !self.lobbies.contains_key(&lobby_id) {
                break;
            };
        }
        lobby_id
    }
}
