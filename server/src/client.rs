use std::collections::hash_map::Entry;
use std::sync::{Arc, RwLock};

use anyhow::Context;
use bfbb::Spatula;
use clash::lobby::GamePhase;
use clash::net::connection::{self, ConnectionRx, ConnectionTx};
use clash::net::{FrameError, Item, Message, ProtocolError};
use clash::PlayerId;
use tokio::net::TcpStream;

use crate::state::State;

/// Accept a new connection from a client and run it's message loop
pub async fn handle_new_connection(state: Arc<RwLock<State>>, socket: TcpStream) {
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    let (conn_tx, conn_rx) = connection::from_socket(socket);
    let client = Client::new(state, conn_rx, tx).await.unwrap();

    tokio::spawn(send_task(conn_tx, rx));

    client.run().await;

    // Player cleanup handled by drop impl
}

async fn broadcast_task(
    mut rx: tokio::sync::broadcast::Receiver<Message>,
    tx: tokio::sync::mpsc::Sender<Message>,
) {
    while let Ok(m) = rx.recv().await {
        tx.send(m).await.unwrap();
    }
}

async fn send_task(mut conn_tx: ConnectionTx, mut rx: tokio::sync::mpsc::Receiver<Message>) {
    while let Some(m) = rx.recv().await {
        conn_tx.write_frame(m).await.unwrap();
    }
}

struct Client {
    state: Arc<RwLock<State>>,
    conn_rx: ConnectionRx,
    tx: tokio::sync::mpsc::Sender<Message>,
    player_id: PlayerId,
}

impl Client {
    pub async fn new(
        state: Arc<RwLock<State>>,
        conn_rx: ConnectionRx,
        tx: tokio::sync::mpsc::Sender<Message>,
    ) -> Result<Self, FrameError> {
        // Add new player
        let player_id = {
            let mut state = state.write().unwrap();
            state.add_player()
        };

        Ok(Self {
            state,
            conn_rx,
            tx,
            player_id,
        })
    }

    pub async fn run(mut self) {
        loop {
            let incoming = match self.conn_rx.read_frame().await {
                Ok(Some(x)) => x,
                Ok(None) => {
                    log::info!("Player id {:#X} disconnected", self.player_id);
                    break;
                }
                Err(e) => {
                    log::error!(
                        "Error reading message from player id {:#X}. Closing connection\n{e:?}",
                        self.player_id
                    );
                    break;
                }
            };

            log::debug!(
                "Received message from player id {:#X} \nMessage: {incoming:#X?}",
                self.player_id,
            );
            if let Err(e) = self.process_incoming(incoming).await {
                match e.downcast_ref::<ProtocolError>() {
                    Some(m @ ProtocolError::InvalidLobbyId(_))
                    | Some(m @ ProtocolError::InvalidMessage) => {
                        log::error!("{e:?}");
                        let _ = self.tx.send(Message::Error { error: m.clone() }).await;
                    }
                    Some(m @ ProtocolError::VersionMismatch(_, _)) => {
                        log::error!("{e:?}");
                        let _ = self.tx.send(Message::Error { error: m.clone() }).await;
                        break;
                    }
                    Some(ProtocolError::Disconnected) => {
                        // Close the connection without error
                        log::info!("Player id {:#X} disconnected", self.player_id);
                        break;
                    }
                    _ => {
                        // This error means there is a problem with our internal state
                        // TODO: Figure out how to properly resolve this error
                        log::error!(
                            "Disconnecting player {:#X} due to unrecoverable error:\n{e:?}",
                            self.player_id
                        );
                        break;
                    }
                }
            }
        }
    }

    async fn process_incoming(&mut self, incoming: Message) -> anyhow::Result<()> {
        match incoming {
            Message::Version { version } => {
                if version != crate::VERSION {
                    return Err(
                        ProtocolError::VersionMismatch(version, crate::VERSION.to_owned()).into(),
                    );
                }

                // Inform player of their PlayerId
                self.tx
                    .send(Message::ConnectionAccept {
                        player_id: self.player_id,
                    })
                    .await?;
                log::info!("New connection for player id {:#X} opened", self.player_id);
            }
            Message::GameHost => {
                let state = &mut *self.state.write().unwrap();
                if state.players.contains_key(&self.player_id) {
                    return Err(ProtocolError::InvalidMessage.into());
                }

                let lobby_id = state.add_lobby();
                let lobby = state
                    .lobbies
                    .get_mut(&lobby_id)
                    .ok_or(ProtocolError::InvalidLobbyId(lobby_id))?;

                let lobby_recv = lobby.add_player(&mut state.players, self.player_id)?;
                tokio::spawn(broadcast_task(lobby_recv, self.tx.clone()));

                log::info!(
                    "Player {:#X} has hosted lobby {:#X}",
                    self.player_id,
                    lobby.shared.lobby_id
                );
            }
            Message::GameJoin { lobby_id } => {
                let state = &mut *self.state.write().unwrap();
                if state.players.contains_key(&self.player_id) {
                    return Err(ProtocolError::InvalidMessage.into());
                }

                let lobby = state
                    .lobbies
                    .get_mut(&lobby_id)
                    .ok_or(ProtocolError::InvalidLobbyId(lobby_id))?;

                let lobby_recv = lobby.add_player(&mut state.players, self.player_id)?;
                tokio::spawn(broadcast_task(lobby_recv, self.tx.clone()));

                log::info!(
                    "Player {:#X} has joined lobby {lobby_id:#X}",
                    self.player_id
                );
            }
            Message::GameBegin => {
                let state = &mut *self.state.write().unwrap();
                let lobby = state.get_lobby(self.player_id)?;
                if lobby.shared.can_start() {
                    lobby.start_game();
                } else {
                    log::warn!(
                        "Lobby {:#X} attempted to start when some players aren't on the Main Menu",
                        lobby.shared.lobby_id
                    )
                }
            }
            Message::GameLeave => {
                // Remove player from their lobby
                let state = &mut *self.state.write().unwrap();

                self.leave_lobby(state);
            }
            Message::PlayerOptions { mut options } => {
                let state = &mut *self.state.write().unwrap();
                let lobby = state.get_lobby(self.player_id)?;

                let player = lobby
                    .shared
                    .players
                    .get_mut(&self.player_id)
                    .ok_or(ProtocolError::InvalidPlayerId(self.player_id))
                    .context("Player not found in lobby specified by the playerlist")?;

                // TODO: Unhardcode player color
                options.color = player.options.color;
                player.options = options;

                let message = Message::GameLobbyInfo {
                    lobby: lobby.shared.clone(),
                };
                let _ = lobby.sender.send(message);
            }
            Message::GameOptions { options } => {
                let state = &mut *self.state.write().unwrap();
                let lobby = state.get_lobby(self.player_id)?;

                if lobby.shared.host_id != Some(self.player_id) {
                    return Err(ProtocolError::InvalidMessage)
                        .context("Only the host can change Lobby Options");
                }
                lobby.shared.options = options;

                let message = Message::GameLobbyInfo {
                    lobby: lobby.shared.clone(),
                };
                let _ = lobby.sender.send(message);
            }
            Message::GameCurrentLevel { level } => {
                let state = &mut *self.state.write().unwrap();
                let lobby = state.get_lobby(self.player_id)?;

                let player = lobby
                    .shared
                    .players
                    .get_mut(&self.player_id)
                    .ok_or(ProtocolError::InvalidPlayerId(self.player_id))
                    .context("Player not found in lobby specified by the playerlist")?;

                player.current_level = level;
                log::info!("Player {:#X} entered {level:?}", self.player_id);

                let message = Message::GameLobbyInfo {
                    lobby: lobby.shared.clone(),
                };
                let _ = lobby.sender.send(message);
            }
            Message::GameItemCollected { item } => {
                let state = &mut *self.state.write().unwrap();
                let lobby = state.get_lobby(self.player_id)?;

                match item {
                    Item::Spatula(spat) => {
                        if let Entry::Vacant(e) = lobby.shared.game_state.spatulas.entry(spat) {
                            e.insert(Some(self.player_id));
                            lobby
                                .shared
                                .players
                                .get_mut(&self.player_id)
                                .ok_or(ProtocolError::InvalidPlayerId(self.player_id))?
                                .score += 1;
                            log::info!("Player {:#X} collected {spat:?}", self.player_id);

                            if spat == Spatula::TheSmallShallRuleOrNot {
                                lobby.shared.game_phase = GamePhase::Finished;
                            }

                            let message = Message::GameLobbyInfo {
                                lobby: lobby.shared.clone(),
                            };
                            let _ = lobby.sender.send(message);
                        }
                    }
                }
            }
            _ => {
                return Err(ProtocolError::InvalidMessage)
                    .context("Client sent a server-only message.")
            }
        }
        Ok(())
    }

    fn leave_lobby(&self, state: &mut State) {
        if let Entry::Occupied(e) = state.players.entry(self.player_id) {
            let lobby_id = e.remove();

            if let Entry::Occupied(mut lobby) = state.lobbies.entry(lobby_id) {
                if lobby.get_mut().rem_player(self.player_id) == 0 {
                    // Remove this lobby from the server
                    log::info!("Closing lobby {:#X}", lobby.key());
                    lobby.remove();
                }
            }
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // This will crash the program if we're dropping due to a previous panic caused by a poisoned lock,
        // and that's fine for now.
        let state = &mut *self.state.write().unwrap();
        self.leave_lobby(state);
    }
}
