use crate::lobby::{LobbyOptions, NetworkedLobby};
use crate::player::PlayerOptions;
use crate::{LobbyId, PlayerId};
use bfbb::{Level, Spatula};
use serde::{Deserialize, Serialize};

use super::ProtocolError;

// TODO: Take more advantage of the type system (e.g. Client/Server messages)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Message {
    Error { error: ProtocolError },
    Version { version: String },
    ConnectionAccept { player_id: PlayerId },
    GameHost,
    GameJoin { lobby_id: LobbyId },
    Lobby(LobbyMessage),
    GameLobbyInfo { lobby: NetworkedLobby },
}

impl From<LobbyMessage> for Message {
    fn from(msg: LobbyMessage) -> Self {
        Self::Lobby(msg)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum LobbyMessage {
    PlayerOptions { options: PlayerOptions },
    PlayerCanStart(bool),
    ResetLobby,
    GameBegin,
    GameEnd,
    GameOptions { options: LobbyOptions },
    GameCurrentLevel { level: Option<Level> },
    GameItemCollected { item: Item },
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum Item {
    Spatula(Spatula),
}

impl From<Spatula> for Item {
    fn from(spat: Spatula) -> Self {
        Self::Spatula(spat)
    }
}
