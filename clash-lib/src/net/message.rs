use crate::lobby::{LobbyOptions, NetworkedLobby};
use crate::player::PlayerOptions;
use bfbb::{Level, Spatula};
use serde::{Deserialize, Serialize};

use super::ProtocolError;

// TODO: Take more advantage of the type system (e.g. Client/Server messages)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Message {
    Error { error: ProtocolError },
    Version { version: String },
    ConnectionAccept { player_id: u32 },
    GameHost,
    GameJoin { lobby_id: u32 },
    Lobby(LobbyMessage),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum LobbyMessage {
    PlayerOptions { options: PlayerOptions },
    PlayerCanStart(bool),
    GameBegin,
    GameEnd,
    GameOptions { options: LobbyOptions },
    GameLobbyInfo { lobby: NetworkedLobby },
    GameCurrentLevel { level: Option<Level> },
    GameItemCollected { item: Item },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Item {
    Spatula(Spatula),
}
