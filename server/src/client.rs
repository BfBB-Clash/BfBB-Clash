use crate::state::State;
use clash::protocol::{self, Connection, Item, Message};
use clash::PlayerId;
use log::{debug, error, info, warn};
use std::collections::hash_map::Entry;
use std::sync::{Arc, RwLock};
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::broadcast::Receiver;

pub struct Client {
    state: Arc<RwLock<State>>,
    connection: Connection,
    player_id: PlayerId,
    lobby_recv: Option<Receiver<Message>>,
}

impl Client {
    pub async fn new(
        state: Arc<RwLock<State>>,
        socket: TcpStream,
    ) -> Result<Self, protocol::Error> {
        // Add new player
        let player_id = {
            let mut state = state.write().expect("Failed to lock State");
            state.add_player()
        };
        let mut connection = Connection::new(socket);

        // Inform player of their PlayerId
        connection
            .write_frame(Message::ConnectionAccept { player_id })
            .await?;
        info!("New connection for player id {player_id:#X} opened");

        Ok(Self {
            state,
            connection,
            player_id,
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
                            info!("Player id {:#X} disconnected", self.player_id);
                            break;
                        }
                        Err(e) => {
                            error!(
                                "Error reading message from player id {:#X}. Closing connection\n{e:?}", self.player_id
                            );
                            break;
                        }
                    };
                    debug!("Received message from player id {:#X} \nMessage: {incoming:#X?}", self.player_id,);
                    if !self.process_incoming(incoming).await {
                        info!("Disconnecting player {:#X} due to unrecoverable error.", self.player_id);
                        break;
                    }
                }
            };
        }
    }

    async fn process_incoming(&mut self, incoming: Message) -> bool {
        match incoming {
            Message::GameHost => {
                let state = &mut *self.state.write().unwrap();
                if state.players.contains_key(&self.player_id) {
                    let lobby_id = state.add_lobby();
                    let lobby = state.lobbies.get_mut(&lobby_id).unwrap();
                    self.lobby_recv = Some(
                        lobby
                            .add_player(&mut state.players, self.player_id)
                            .unwrap(),
                    );

                    info!(
                        "Player {:#X} has hosted lobby {:#X}",
                        self.player_id, lobby.shared.lobby_id
                    );
                    lobby
                        .sender
                        .send(Message::GameLobbyInfo {
                            lobby: lobby.shared.clone(),
                        })
                        .unwrap();
                }
            }
            Message::GameJoin { lobby_id } => {
                let state = &mut *self.state.write().unwrap();

                if state.players.contains_key(&self.player_id) {
                    let lobby = match state.lobbies.get_mut(&lobby_id) {
                        None => {
                            error!("Attempted to join lobby with an invalid id '{lobby_id}'");
                            return true;
                        }
                        Some(l) => l,
                    };

                    self.lobby_recv = Some(
                        lobby
                            .add_player(&mut state.players, self.player_id)
                            .unwrap(),
                    );

                    info!(
                        "Player {:#X} has joined lobby {lobby_id:#X}",
                        self.player_id
                    );
                    lobby
                        .sender
                        .send(Message::GameLobbyInfo {
                            lobby: lobby.shared.clone(),
                        })
                        .unwrap();
                } else {
                    error!("Player {:#X} not in the playerlist", self.player_id);
                }
            }
            Message::GameLobbyInfo { lobby: _ } => todo!(),
            Message::GameBegin => {
                let state = &mut *self.state.write().unwrap();

                let lobby_id = match state.players.get(&self.player_id) {
                    Some(Some(id)) => id,
                    Some(None) => {
                        error!(
                            "Player id {:#X} attempted to start a game while not in a lobby",
                            self.player_id
                        );
                        return true;
                    }
                    None => {
                        error!("Invalid player id {:#X}", self.player_id);
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

                lobby.start_game();
            }
            Message::GameEnd => todo!(),
            Message::GameLeave => {
                // Disconnect player
                info!("Player {:#X} disconnecting.", self.player_id);
                return false;
            }
            Message::PlayerOptions { mut options } => {
                let state = &mut *self.state.write().unwrap();

                let lobby_id = match state.players.get(&self.player_id) {
                    Some(Some(id)) => id,
                    Some(None) => {
                        error!("Player id {:#X} not currently in a lobby", self.player_id);
                        return true;
                    }
                    None => {
                        error!("Invalid player id {:#X}", self.player_id);
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

                if let Some(player) = lobby.shared.players.get_mut(&self.player_id) {
                    // TODO: Unhardcode player color
                    options.color = player.options.color;
                    player.options = options;
                } else {
                    error!(
                        "Lobby received from player map did not contain player {:#X}",
                        self.player_id
                    );
                    return false;
                }

                let message = Message::GameLobbyInfo {
                    lobby: lobby.shared.clone(),
                };
                lobby.sender.send(message).unwrap();
            }
            Message::GameOptions { options } => {
                let state = &mut *self.state.write().unwrap();
                let lobby_id = match state.players.get(&self.player_id) {
                    Some(Some(id)) => id,
                    Some(None) => {
                        error!("Player id {:#X} not currently in a lobby", self.player_id);
                        return true;
                    }
                    None => {
                        error!("Invalid player id {:#X}", self.player_id);
                        //TODO: Ditto
                        return false;
                    }
                };

                let lobby = match state.lobbies.get_mut(lobby_id) {
                    Some(l) => {
                        if l.shared.host_id == Some(self.player_id) {
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
                    lobby: lobby.shared.clone(),
                };
                lobby.sender.send(message).unwrap();
            }
            Message::GameCurrentRoom { room } => {
                let state = &mut *self.state.write().unwrap();

                let lobby_id = match state.players.get_mut(&self.player_id) {
                    Some(Some(id)) => id,
                    Some(None) => {
                        error!("Player id {:#X} not currently in a lobby", self.player_id);
                        return true;
                    }
                    None => {
                        error!("Invalid player id {:#X}", self.player_id);
                        return false;
                    }
                };

                let lobby = match state.lobbies.get_mut(lobby_id) {
                    Some(l) => {
                        l.shared
                            .players
                            .get_mut(&self.player_id)
                            .unwrap_or_else(|| {
                                panic!(
                                    "Lobby received from player map did not contain player {:#X}",
                                    self.player_id,
                                )
                            })
                            .current_room = room;
                        info!("Player {:#X} entered {room:?}", self.player_id);
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
                        lobby: lobby.shared.clone(),
                    })
                    .unwrap();
            }
            Message::GameItemCollected { item } => {
                let state = &mut *self.state.write().unwrap();

                let lobby_id = match state.players.get_mut(&self.player_id) {
                    Some(Some(id)) => id,
                    Some(None) => {
                        error!("Player id {:#X} not currently in a lobby", self.player_id);
                        return true;
                    }
                    None => {
                        error!("Invalid player id {:#X}", self.player_id);
                        return false;
                    }
                };

                let lobby = match state.lobbies.get_mut(lobby_id) {
                    Some(l) => {
                        if let Item::Spatula(spat) = item {
                            if let Entry::Vacant(e) = l.shared.game_state.spatulas.entry(spat) {
                                e.insert(Some(self.player_id));
                                l.shared.players
                                .get_mut(&self.player_id)
                                .unwrap_or_else(|| panic!("Lobby received from player map did not contain player {:#X}", self.player_id))
                                .score += 1;
                                info!("Player {:#X} collected {spat:?}", self.player_id);
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
                        lobby: lobby.shared.clone(),
                    })
                    .unwrap();
            }
            m => {
                warn!(
                    "Player id {:#X} sent a server only message. \nMessage: {m:?}",
                    self.player_id
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
        if let Entry::Occupied(e) = state.players.entry(self.player_id) {
            let lobby_id = match e.remove() {
                Some(it) => it,
                None => return,
            };

            if let Entry::Occupied(mut lobby) = state.lobbies.entry(lobby_id) {
                if lobby.get_mut().rem_player(self.player_id) == 0 {
                    // Remove this lobby from the server
                    info!("Closing lobby {:#X}", lobby.key());
                    lobby.remove();
                }
            }
        }
    }
}
