use crate::state::State;
use anyhow::Context;
use clash::lobby::GamePhase;
use clash::protocol::{self, Connection, Item, Message};
use clash::spatula::Spatula;
use clash::{LobbyId, PlayerId};
use std::collections::hash_map::Entry;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::broadcast::Receiver;

#[derive(Error, Clone, Debug)]
pub enum Error {
    #[error("Invalid Player ID {0:#X}")]
    InvalidPlayerId(PlayerId),
    #[error("Invalid Lobby ID {0:#X}")]
    InvalidLobbyId(LobbyId),
    #[error("Invalid Message")]
    InvalidMessage,
    // TODO: This probably shouldn't be an error
    #[error("Player disconnected")]
    Disconnected,
    #[error(
        "Client version '{0}' does not match server version '{}'",
        crate::VERSION
    )]
    VersionMismatch(String),
}

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
            let mut state = state.write().unwrap();
            state.add_player()
        };

        Ok(Self {
            state,
            connection: Connection::new(socket),
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
                            log::info!("Player id {:#X} disconnected", self.player_id);
                            break;
                        }
                        Err(e) => {
                            log::error!(
                                "Error reading message from player id {:#X}. Closing connection\n{e:?}", self.player_id
                            );
                            break;
                        }
                    };

                    log::debug!("Received message from player id {:#X} \nMessage: {incoming:#X?}", self.player_id,);
                    if let Err(e) = self.process_incoming(incoming).await {
                        match e.downcast_ref::<Error>() {
                            Some(Error::InvalidLobbyId(_)) => {
                                // Usually this is the result of the client joining an invalid lobby id
                                // TODO: Inform client of failure
                                log::error!("{e:?}");
                            }
                            Some(Error::InvalidMessage) => {
                                // TODO: Tell the client their message was invalid?
                                log::error!("{e:?}");
                            },
                            Some(Error::Disconnected) => {
                                // Close the connection without error
                                log::info!("Player id {:#X} disconnected", self.player_id);
                                break;
                            }
                            _ => {
                                // This error means there is a problem with our internal state
                                // TODO: Figure out how to properly resolve this error
                                log::error!("Disconnecting player {:#X} due to unrecoverable error.", self.player_id);
                                log::error!("{e:?}");
                                break;
                            }
                        }
                    }
                }
            };
        }
    }

    async fn process_incoming(&mut self, incoming: Message) -> anyhow::Result<()> {
        match incoming {
            Message::Version { version } => {
                if version != crate::VERSION {
                    // Tell the client what the correct version is and disconnect
                    self.connection
                        .write_frame(Message::Version {
                            version: crate::VERSION.to_owned(),
                        })
                        .await?;
                    return Err(Error::VersionMismatch(version).into());
                }

                // Inform player of their PlayerId
                self.connection
                    .write_frame(Message::ConnectionAccept {
                        player_id: self.player_id,
                    })
                    .await?;
                log::info!("New connection for player id {:#X} opened", self.player_id);
            }
            Message::GameHost => {
                let state = &mut *self.state.write().unwrap();
                if state.players.contains_key(&self.player_id) {
                    let lobby_id = state.add_lobby();
                    let lobby = state
                        .lobbies
                        .get_mut(&lobby_id)
                        .ok_or(Error::InvalidLobbyId(lobby_id))?;

                    self.lobby_recv = Some(lobby.add_player(&mut state.players, self.player_id)?);

                    log::info!(
                        "Player {:#X} has hosted lobby {:#X}",
                        self.player_id,
                        lobby.shared.lobby_id
                    );
                }
            }
            Message::GameJoin { lobby_id } => {
                let state = &mut *self.state.write().unwrap();
                let lobby = state
                    .lobbies
                    .get_mut(&lobby_id)
                    .ok_or(Error::InvalidLobbyId(lobby_id))?;

                self.lobby_recv = Some(lobby.add_player(&mut state.players, self.player_id)?);

                log::info!(
                    "Player {:#X} has joined lobby {lobby_id:#X}",
                    self.player_id
                );
            }
            Message::GameBegin => {
                let state = &mut *self.state.write().unwrap();
                let lobby = state.get_lobby(self.player_id)?;
                lobby.start_game();
            }
            Message::GameLeave => {
                // Disconnect player
                log::info!("Player {:#X} disconnecting.", self.player_id);
                return Err(Error::Disconnected.into());
            }
            Message::PlayerOptions { mut options } => {
                let state = &mut *self.state.write().unwrap();
                let lobby = state.get_lobby(self.player_id)?;

                let player = lobby
                    .shared
                    .players
                    .get_mut(&self.player_id)
                    .ok_or(Error::InvalidPlayerId(self.player_id))
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
                    return Err(Error::InvalidMessage)
                        .context("Only the host can change Lobby Options");
                }
                lobby.shared.options = options;

                let message = Message::GameLobbyInfo {
                    lobby: lobby.shared.clone(),
                };
                let _ = lobby.sender.send(message);
            }
            Message::GameCurrentRoom { room } => {
                let state = &mut *self.state.write().unwrap();
                let lobby = state.get_lobby(self.player_id)?;

                let player = lobby
                    .shared
                    .players
                    .get_mut(&self.player_id)
                    .ok_or(Error::InvalidPlayerId(self.player_id))
                    .context("Player not found in lobby specified by the playerlist")?;

                player.current_room = room;
                log::info!("Player {:#X} entered {room:?}", self.player_id);

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
                                .ok_or(Error::InvalidPlayerId(self.player_id))?
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
            _ => return Err(Error::InvalidMessage).context("Client sent a server-only message."),
        }
        Ok(())
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
                    log::info!("Closing lobby {:#X}", lobby.key());
                    lobby.remove();
                }
            }
        }
    }
}
