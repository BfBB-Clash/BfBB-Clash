use crate::LobbyId;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Clone, Debug, Serialize, Deserialize)]
pub enum ProtocolError {
    #[error("Invalid Lobby ID {0:#X}")]
    InvalidLobbyId(LobbyId),
    #[error("Invalid Message")]
    InvalidMessage,
    // TODO: This probably shouldn't be an error
    #[error("Player disconnected")]
    Disconnected,
    #[error("Client version '{0}' does not match server version '{1}'")]
    VersionMismatch(String, String),
    #[error("{0}")]
    Message(String),
}

impl From<FrameError> for ProtocolError {
    fn from(e: FrameError) -> Self {
        Self::Message(e.to_string())
    }
}

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("Frame exceeded max length")]
    FrameLength,
    #[error("Full frame is not available yet.")]
    FrameIncomplete,
    #[error("Connection reset by peer")]
    ConnectionReset,
    #[error("I/O Error: {0}")]
    Io(std::io::Error),
    #[error("Serialization Error: {0}")]
    Bincode(bincode::Error),
}

impl From<std::io::Error> for FrameError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<bincode::Error> for FrameError {
    fn from(e: bincode::Error) -> Self {
        Self::Bincode(e)
    }
}
