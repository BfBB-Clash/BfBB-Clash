use clash_lib::{LobbyId, PlayerId};
use thiserror::Error;
use tokio::sync::mpsc;

use crate::state::ServerState;

use self::{lobby_actor::LobbyActor, lobby_handle::LobbyHandle};

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

pub type LobbyResult<T> = Result<T, LobbyError>;

pub fn start_new_lobby(state: ServerState, id: LobbyId) -> LobbyHandle {
    let (sender, receiver) = mpsc::channel(64);
    let actor = LobbyActor::new(state, receiver, id);
    tokio::spawn(actor.run());

    LobbyHandle {
        sender,
        lobby_id: id,
        // TODO: We need to store a handle on the state but there's no player_id then, using an Option would be
        // unergonomic so this should suffice for now. Any errors resulting from this should be caught by the lobby_actor
        // Maybe we need a LobbyHandleProvider type that stores the sender and hands out LobbyHandles
        player_id: 0,
    }
}
