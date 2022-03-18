use crate::lobby::Lobby;
use crate::state::State;
use clash::lobby::LobbyOptions;
use clash::protocol::{self, Connection, Item, Message};
use clash::AuthId;
use log::{debug, error, info, warn};
use std::collections::hash_map::Entry;
use std::sync::{Arc, RwLock};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::broadcast::Receiver;

pub struct Client {
    state: Arc<RwLock<State>>,
    connection: Connection,
    auth_id: AuthId,
    lobby_recv: Option<Receiver<Message>>,
}

impl Client {
    pub async fn new(
        state: Arc<RwLock<State>>,
        socket: TcpStream,
    ) -> Result<Self, protocol::Error> {
        // Add new player
        let auth_id = {
            let mut state = state.write().expect("Failed to lock State");
            state.add_player()
        };
        let mut connection = Connection::new(socket);

        // Inform player of their auth_id
        connection
            .write_frame(Message::ConnectionAccept { auth_id })
            .await?;
        info!("New connection for player id {auth_id:#X} opened");

        Ok(Self {
            state,
            connection,
            auth_id,
            lobby_recv: None,
        })
    }

    pub async fn run(mut self) {
        loop {
            select! {
                m = async { self.lobby_recv.as_mut().unwrap().recv().await }, if self.lobby_recv.is_some() => {
                    self.connection.write_frame(m.unwrap()).await.unwrap();
                }
                incoming = self.connection.read_frame() => {
                    let incoming = match incoming {
                        Ok(Some(x)) => x,
                        Ok(None) => {
                            info!("Player id {:#X} disconnected", self.auth_id);
                            break;
                        }
                        Err(e) => {
                            error!(
                                "Error reading message from player id {:#X}. Closing connection\n{e:?}", self.auth_id
                            );
                            break;
                        }
                    };
                    debug!("Received message from player id {:#X} \nMessage: {incoming:#X?}", self.auth_id,);
                    if !self.process_incoming(incoming).await {
                        info!("Disconnecting player {:#X} due to unrecoverable error.", self.auth_id);
                        break;
                    }
                }
            };
        }
    }

    async fn process_incoming(&mut self, incoming: Message) -> bool {
        match incoming {
            Message::GameHost { auth_id: _ } => {
                let state = &mut *self.state.write().unwrap();
                if state.players.contains_key(&self.auth_id) {
                    let gen_lobby_id = state.gen_lobby_id();
                    state.lobbies.insert(
                        gen_lobby_id,
                        Lobby::new(LobbyOptions::default(), gen_lobby_id, self.auth_id),
                    );

                    let lobby = match state.lobbies.get_mut(&gen_lobby_id) {
                        None => {
                            error!("Attempted to join lobby with an invalid id '{gen_lobby_id}'");
                            return true;
                        }
                        Some(l) => l,
                    };

                    self.lobby_recv =
                        Some(lobby.add_player(&mut state.players, self.auth_id).unwrap());

                    info!(
                        "Player {:#X} has hosted lobby {gen_lobby_id:#X}",
                        self.auth_id
                    );
                    lobby
                        .sender
                        .send(Message::GameLobbyInfo {
                            auth_id: 0,
                            lobby: lobby.shared.clone(),
                        })
                        .unwrap();
                }
            }
            Message::GameJoin {
                auth_id: _,
                lobby_id,
            } => {
                let state = &mut *self.state.write().unwrap();

                if state.players.contains_key(&self.auth_id) {
                    let lobby = match state.lobbies.get_mut(&lobby_id) {
                        None => {
                            error!("Attempted to join lobby with an invalid id '{lobby_id}'");
                            return true;
                        }
                        Some(l) => l,
                    };

                    self.lobby_recv =
                        Some(lobby.add_player(&mut state.players, self.auth_id).unwrap());

                    info!("Player {:#X} has joined lobby {lobby_id:#X}", self.auth_id);
                    lobby
                        .sender
                        .send(Message::GameLobbyInfo {
                            auth_id: 0,
                            lobby: lobby.shared.clone(),
                        })
                        .unwrap();
                } else {
                    error!("Player {:#X} not in the playerlist", self.auth_id);
                }
            }
            Message::GameLobbyInfo {
                auth_id: _,
                lobby: _,
            } => todo!(),
            Message::GameBegin { auth_id: _ } => {
                let state = &mut *self.state.write().unwrap();

                let lobby_id = match state.players.get(&self.auth_id) {
                    Some(Some(id)) => id,
                    Some(None) => {
                        error!(
                            "Player id {:#X} attempted to start a game while not in a lobby",
                            self.auth_id
                        );
                        return true;
                    }
                    None => {
                        error!("Invalid player id {:#X}", self.auth_id);
                        //TODO: Kick player?
                        return false;
                    }
                };

                let lobby = match state.lobbies.get_mut(lobby_id) {
                    None => {
                        error!("Attempted to start game in lobby with an invalid id '{lobby_id}'");
                        return true;
                    }
                    Some(l) => l,
                };

                lobby.shared.is_started = true;

                lobby
                    .sender
                    .send(Message::GameBegin { auth_id: 0 })
                    .unwrap();
            }
            Message::GameEnd { auth_id: _ } => todo!(),
            Message::GameLeave { auth_id: _ } => {
                let state = &mut *self.state.write().unwrap();

                let lobby_id = match state.players.get(&self.auth_id) {
                    Some(Some(id)) => id,
                    Some(None) => {
                        error!(
                            "Player id {:#X} attempted to leave a lobby while not in a lobby",
                            self.auth_id
                        );
                        return true;
                    }
                    None => {
                        error!("Invalid player id {:#X}", self.auth_id);
                        //TODO: Kick player?
                        return false;
                    }
                };

                let lobby = match state.lobbies.get_mut(lobby_id) {
                    None => {
                        error!("Attempted to leave lobby with an invalid id '{lobby_id}'");
                        return true;
                    }
                    Some(l) => l,
                };

                if lobby.is_player_in_lobby(&self.auth_id) {
                    lobby.rem_player(&mut state.players, &self.auth_id);
                }
            }
            Message::PlayerOptions {
                auth_id: _,
                mut options,
            } => {
                let state = &mut *self.state.write().unwrap();

                let lobby_id = match state.players.get(&self.auth_id) {
                    Some(Some(id)) => id,
                    Some(None) => {
                        error!("Player id {:#X} not currently in a lobby", self.auth_id);
                        return true;
                    }
                    None => {
                        error!("Invalid player id {:#X}", self.auth_id);
                        //TODO: Kick player?
                        return false;
                    }
                };

                let lobby = match state.lobbies.get_mut(lobby_id) {
                    Some(l) => l,
                    None => {
                        error!("Invalid lobby id received from player map {:#X}", lobby_id);
                        return true;
                    }
                };

                if let Some(player) = lobby.shared.players.get_mut(&self.auth_id) {
                    // TODO: Unhardcode player color
                    options.color = player.options.color;
                    player.options = options;
                } else {
                    error!(
                        "Lobby received from player map did not contain player {:#X}",
                        self.auth_id
                    );
                    return false;
                }

                let message = Message::GameLobbyInfo {
                    auth_id: 0,
                    lobby: lobby.shared.clone(),
                };
                lobby.sender.send(message).unwrap();
            }
            Message::GameOptions {
                auth_id: _,
                options,
            } => {
                let state = &mut *self.state.write().unwrap();
                let lobby_id = match state.players.get(&self.auth_id) {
                    Some(Some(id)) => id,
                    Some(None) => {
                        error!("Player id {:#X} not currently in a lobby", self.auth_id);
                        return true;
                    }
                    None => {
                        error!("Invalid player id {:#X}", self.auth_id);
                        //TODO: Ditto
                        return false;
                    }
                };

                let lobby = match state.lobbies.get_mut(lobby_id) {
                    Some(l) => {
                        if l.shared.host_id == self.auth_id {
                            l.shared.options = options;
                        }
                        l
                    }
                    None => {
                        error!("Invalid lobby id received from player map {:#X}", lobby_id);
                        return true;
                    }
                };

                let message = Message::GameLobbyInfo {
                    auth_id: 0,
                    lobby: lobby.shared.clone(),
                };
                lobby.sender.send(message).unwrap();
            }
            Message::GameCurrentRoom { auth_id: _, room } => {
                let state = &mut *self.state.write().unwrap();

                let lobby_id = match state.players.get_mut(&self.auth_id) {
                    Some(Some(id)) => id,
                    Some(None) => {
                        error!("Player id {:#X} not currently in a lobby", self.auth_id);
                        return true;
                    }
                    None => {
                        error!("Invalid player id {:#X}", self.auth_id);
                        return false;
                    }
                };

                let lobby = match state.lobbies.get_mut(lobby_id) {
                    Some(l) => {
                        l.shared
                            .players
                            .get_mut(&self.auth_id)
                            .unwrap_or_else(|| {
                                panic!(
                                    "Lobby received from player map did not contain player {:#X}",
                                    self.auth_id,
                                )
                            })
                            .current_room = room;
                        info!("Player {:#X} entered {room:?}", self.auth_id);
                        l
                    }
                    None => {
                        error!("Invalid lobby id {lobby_id:#X}");
                        return false;
                    }
                };

                lobby
                    .sender
                    .send(Message::GameLobbyInfo {
                        auth_id: 0,
                        lobby: lobby.shared.clone(),
                    })
                    .unwrap();
            }
            Message::GameItemCollected { auth_id: _, item } => {
                let state = &mut *self.state.write().unwrap();

                let lobby_id = match state.players.get_mut(&self.auth_id) {
                    Some(Some(id)) => id,
                    Some(None) => {
                        error!("Player id {:#X} not currently in a lobby", self.auth_id);
                        return true;
                    }
                    None => {
                        error!("Invalid player id {:#X}", self.auth_id);
                        return false;
                    }
                };

                let lobby = match state.lobbies.get_mut(lobby_id) {
                    Some(l) => {
                        if let Item::Spatula(spat) = item {
                            if let Entry::Vacant(e) = l.shared.game_state.spatulas.entry(spat) {
                                e.insert(Some(self.auth_id));
                                l.shared.players
                                .get_mut(&self.auth_id)
                                .unwrap_or_else(|| panic!("Lobby received from player map did not contain player {:#X}", self.auth_id))
                                .score += 1;
                                info!("Player {:#X} collected {spat:?}", self.auth_id);
                            }
                        }
                        l
                    }
                    None => {
                        error!("Invalid lobby id {:#X}", lobby_id);
                        return false;
                    }
                };

                lobby
                    .sender
                    .send(Message::GameLobbyInfo {
                        auth_id: 0,
                        lobby: lobby.shared.clone(),
                    })
                    .unwrap();
            }
            m => {
                warn!(
                    "Player id {:#X} sent a server only message. \nMessage: {m:?}",
                    self.auth_id
                );
            }
        }
        true
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // This will crash the program if we're dropping due to a previous panic caused by a poisoned lock,
        // and that's fine for now.
        let state = &mut *self.state.write().unwrap();
        if let Entry::Occupied(e) = state.players.entry(self.auth_id) {
            let lobby_id = match e.remove() {
                Some(it) => it,
                None => return,
            };

            if let Some(lobby) = state.lobbies.get_mut(&lobby_id) {
                lobby.rem_player(&mut state.players, &self.auth_id);
            }
        }
    }
}
