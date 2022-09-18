use crate::lobby::{LobbyOptions, NetworkedLobby};
use crate::player::PlayerOptions;
use bfbb::{Level, Spatula};
use serde::{Deserialize, Serialize};

use super::ProtocolError;

// TODO: Take more advantage of the type system (e.g. Client/Server messages)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Message {
    Version { version: String },
    Error { error: ProtocolError },
    ConnectionAccept { player_id: u32 },
    PlayerOptions { options: PlayerOptions },
    PlayerCanStart(bool),
    GameHost,
    GameJoin { lobby_id: u32 },
    GameOptions { options: LobbyOptions },
    GameLobbyInfo { lobby: NetworkedLobby },
    GameBegin,
    GameCurrentLevel { level: Option<Level> },
    GameForceWarp { level: Level },
    GameItemCollected { item: Item },
    GameEnd,
    GameLeave,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Item {
    Spatula(Spatula),
}
