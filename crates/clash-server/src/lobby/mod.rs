use clash_lib::{net::ProtocolError, LobbyId, PlayerId};
use thiserror::Error;
use tokio::sync::mpsc;

use crate::state::OwnedId;

use self::{
    lobby_actor::LobbyActor,
    lobby_handle::{LobbyHandle, LobbyHandleProvider},
};

mod lobby_actor;
pub mod lobby_handle;

#[derive(Copy, Clone, Debug, Error, PartialEq, Eq)]
pub enum LobbyError {
    #[error("Attempted to add a player to a full lobby")]
    LobbyFull,
    #[error("Action attempted by Player {0:#} who is not in this lobby")]
    PlayerInvalid(PlayerId),
    #[error("Action attempted by Player {0:#} was invalid.")]
    InvalidAction(PlayerId),
    #[error("Non-host attempted a host-only action")]
    NeedsHost,
    #[error("The Lobby Handle is no longer connected to a lobby.")]
    HandleInvalid,
}

impl From<LobbyError> for ProtocolError {
    fn from(v: LobbyError) -> Self {
        Self::Message(v.to_string())
    }
}

pub type LobbyResult<T> = Result<T, LobbyError>;

pub fn start_new_lobby(
    id: OwnedId<LobbyId>,
    host_id: PlayerId,
) -> (LobbyHandleProvider, LobbyHandle) {
    let (sender, receiver) = mpsc::channel(64);
    let weak_sender = sender.downgrade();
    let actor = LobbyActor::new(receiver, id);
    let handle = LobbyHandle {
        sender,
        player_id: host_id,
    };
    tokio::spawn(actor.run());

    (
        LobbyHandleProvider {
            sender: weak_sender,
        },
        handle,
    )
}
