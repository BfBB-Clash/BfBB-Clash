use clash_lib::net::ProtocolError;
use clash_lib::{LobbyId, PlayerId};
use rand::{thread_rng, Rng};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::lobby;
use crate::lobby::lobby_handle::{LobbyHandle, LobbyHandleProvider};

#[derive(Clone, Debug, Default)]
pub struct ServerState {
    players: Arc<Mutex<HashSet<PlayerId>>>,
    lobbies: Arc<Mutex<HashMap<LobbyId, LobbyHandleProvider>>>,
}

impl ServerState {
    pub fn add_player(&self) -> OwnedId<PlayerId> {
        let player_id = self.gen_player_id();
        self.players().insert(player_id);
        OwnedId::<PlayerId>::new(self.clone(), player_id)
    }

    /// Open a new lobby with the player represented by `host_id` as the only player.
    ///
    /// This will add a [`LobbyHandleProvider`] to [`ServerState`]'s lobby list and return a
    /// concrete `LobbyHandle` for the player who opened the lobby.
    pub fn open_lobby(&self, host_id: PlayerId) -> LobbyHandle {
        let lobby_id = self.gen_lobby_id();
        let (handle_provider, handle) =
            lobby::start_new_lobby(OwnedId::<LobbyId>::new(self.clone(), lobby_id), host_id);
        tracing::info!("Lobby {lobby_id} opened");
        self.lobbies().insert(lobby_id, handle_provider);
        handle
    }

    /// Get a [`LobbyHandleProvider`] instance for the specified `lobby_id`
    ///
    /// # Errors
    ///
    /// Will return a [`ProtocolError::InvalidLobbyId`] if the given lobby id does
    /// not correspond to an open lobby.
    pub fn get_lobby_handle_provider(
        &self,
        lobby_id: LobbyId,
    ) -> Result<LobbyHandleProvider, ProtocolError> {
        let provider = self
            .lobbies()
            .get_mut(&lobby_id)
            .ok_or(ProtocolError::InvalidLobbyId(lobby_id))?
            .clone();
        Ok(provider)
    }

    fn players(&self) -> MutexGuard<HashSet<PlayerId>> {
        self.players.lock().unwrap()
    }

    fn lobbies(&self) -> MutexGuard<HashMap<LobbyId, LobbyHandleProvider>> {
        self.lobbies.lock().unwrap()
    }

    // TODO: dedupe this.
    fn gen_player_id(&self) -> PlayerId {
        let mut player_id;
        loop {
            player_id = thread_rng().gen::<u32>().into();
            if !self.players().contains(&player_id) {
                break;
            };
        }
        player_id
    }

    fn gen_lobby_id(&self) -> LobbyId {
        let mut lobby_id;
        loop {
            lobby_id = thread_rng().gen::<u32>().into();
            if !self.lobbies().contains_key(&lobby_id) {
                break;
            };
        }
        lobby_id
    }
}

/// Wrapper around Id types that is handed out when an Id is stored in the state
/// and when dropped will remove that id from the state.
#[derive(Debug)]
pub struct OwnedId<Id: Copy> {
    state: ServerState,
    id: Id,
    cleanup: fn(ServerState, Id),
}

impl<Id: Display + Copy> Display for OwnedId<Id> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.id.fmt(f)
    }
}

/// Workaround for testing LobbyActor
#[cfg(test)]
impl From<LobbyId> for OwnedId<LobbyId> {
    fn from(v: LobbyId) -> Self {
        Self {
            state: ServerState::default(),
            id: v,
            cleanup: |_, _| {},
        }
    }
}

impl OwnedId<PlayerId> {
    fn new(state: ServerState, id: PlayerId) -> Self {
        Self {
            state,
            id,
            cleanup: |state, id| {
                tracing::info!("Player disconnected");
                state.players.lock().unwrap().remove(&id);
            },
        }
    }
}

impl OwnedId<LobbyId> {
    fn new(state: ServerState, id: LobbyId) -> Self {
        Self {
            state,
            id,
            cleanup: |state, id| {
                tracing::info!("Closing lobby");
                state.lobbies.lock().unwrap().remove(&id);
            },
        }
    }
}

impl<Id: Copy> Deref for OwnedId<Id> {
    type Target = Id;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl<Id: Copy> Drop for OwnedId<Id> {
    fn drop(&mut self) {
        // This will crash the program if we're dropping due to a previous panic caused by a poisoned lock,
        // and that's fine for now.
        (self.cleanup)(self.state.clone(), self.id);
    }
}
