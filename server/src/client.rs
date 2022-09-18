use anyhow::Context;
use clash::net::connection::{self, ConnectionRx, ConnectionTx};
use clash::net::{Message, ProtocolError};
use clash::PlayerId;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::lobby::lobby_handle::LobbyHandle;
use crate::state::ServerState;

/// Accept a new connection from a client and run it's message loop
pub async fn handle_new_connection(state: ServerState, socket: TcpStream) {
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    let (conn_tx, conn_rx) = connection::from_socket(socket);
    let client = Client::new(state, conn_rx, tx);

    tokio::spawn(send_task(conn_tx, rx));
    tokio::spawn(client.run());

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
    state: ServerState,
    conn_rx: ConnectionRx,
    tx: mpsc::Sender<Message>,
    player_id: PlayerId,
    lobby: Option<LobbyHandle>,
}

impl Client {
    pub fn new(
        state: ServerState,
        conn_rx: ConnectionRx,
        tx: tokio::sync::mpsc::Sender<Message>,
    ) -> Self {
        // Add new player
        let player_id = {
            let mut state = state.lock().unwrap();
            state.add_player()
        };

        Self {
            state,
            conn_rx,
            tx,
            player_id,
            lobby: None,
        }
    }

    /// Takes ownership of self to guarantee that client will be dropped when it's
    /// message loop ends
    pub async fn run(mut self) {
        loop {
            let incoming = match self.conn_rx.read_frame().await {
                Ok(Some(x)) => x,
                Ok(None) => {
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
        log::info!("Player id {:#X} disconnected", self.player_id);
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
                let lobby_handle = {
                    let state = &mut *self.state.lock().unwrap();
                    state.players.insert(self.player_id);
                    state.add_lobby(self.state.clone())
                };
                let lobby_recv = lobby_handle.add_player(self.player_id).await?;
                tokio::spawn(broadcast_task(lobby_recv, self.tx.clone()));

                log::info!(
                    "Player {:#X} has hosted lobby {:#X}",
                    self.player_id,
                    lobby_handle.get_lobby_id()
                );
                self.lobby = Some(lobby_handle);
            }
            Message::GameJoin { lobby_id } => {
                let lobby_handle = {
                    let state = &mut *self.state.lock().unwrap();
                    state.players.insert(self.player_id);
                    state
                        .lobbies
                        .get_mut(&lobby_id)
                        .ok_or(ProtocolError::InvalidLobbyId(lobby_id))?
                        .clone()
                };

                let lobby_recv = lobby_handle.add_player(self.player_id).await?;
                tokio::spawn(broadcast_task(lobby_recv, self.tx.clone()));

                log::info!(
                    "Player {:#X} has joined lobby {lobby_id:#X}",
                    self.player_id
                );
                self.lobby = Some(lobby_handle);
            }
            Message::GameBegin => {
                let lobby = self.lobby.as_mut().ok_or(ProtocolError::InvalidMessage)?;
                lobby.start_game(self.player_id).await?;
            }
            Message::GameLeave => {
                let lobby = self.lobby.as_mut().ok_or(ProtocolError::InvalidMessage)?;
                lobby.rem_player(self.player_id).await?;
                self.lobby = None;
                // TODO: Abort broadcast receiver task (preferably by closing connection)
            }
            Message::PlayerOptions { options } => {
                let lobby = self.lobby.as_mut().ok_or(ProtocolError::InvalidMessage)?;
                lobby.set_player_options(self.player_id, options).await?;
            }
            Message::GameOptions { options } => {
                let lobby = self.lobby.as_mut().ok_or(ProtocolError::InvalidMessage)?;
                lobby.set_game_options(self.player_id, options).await?;
            }
            Message::GameCurrentLevel { level } => {
                let lobby = self.lobby.as_mut().ok_or(ProtocolError::InvalidMessage)?;
                lobby.set_player_level(self.player_id, level).await?;
            }
            Message::GameItemCollected { item } => {
                let lobby = self.lobby.as_mut().ok_or(ProtocolError::InvalidMessage)?;
                lobby.player_collected_item(self.player_id, item).await?;
            }
            _ => {
                return Err(ProtocolError::InvalidMessage)
                    .context("Client sent a server-only message.")
            }
        }
        Ok(())
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Some(lobby) = self.lobby.take() {
            let player_id = self.player_id;
            tokio::spawn(async move { lobby.rem_player(player_id).await });
        }
        // This will crash the program if we're dropping due to a previous panic caused by a poisoned lock,
        // and that's fine for now.
        let state = &mut *self.state.lock().unwrap();
        state.players.remove(&self.player_id);
    }
}
