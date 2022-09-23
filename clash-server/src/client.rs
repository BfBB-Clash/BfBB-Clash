use std::mem::ManuallyDrop;

use clash_lib::net::connection::{self, ConnectionRx, ConnectionTx};
use clash_lib::net::{Message, ProtocolError};
use clash_lib::PlayerId;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::lobby::lobby_handle::LobbyHandle;
use crate::lobby::LobbyError;
use crate::state::ServerState;

/// Accept a new connection from a client and run it's message loop
pub async fn handle_new_connection(state: ServerState, socket: TcpStream) {
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    let (conn_tx, conn_rx) = connection::from_socket(socket);
    let client = Client::new(state, conn_rx, tx);

    tokio::spawn(async move {
        let send_handle = tokio::spawn(send_task(conn_tx, rx));
        client.run().await;
        send_handle.abort();
    });

    // Player cleanup handled by drop impl
}

async fn broadcast_task(
    mut rx: tokio::sync::broadcast::Receiver<Message>,
    tx: tokio::sync::mpsc::Sender<Message>,
) {
    while let Ok(m) = rx.recv().await {
        if tx.send(m).await.is_err() {
            // The client has disconnected and the send_task has completed.
            break;
        }
    }
}

async fn send_task(mut conn_tx: ConnectionTx, mut rx: tokio::sync::mpsc::Receiver<Message>) {
    while let Some(m) = rx.recv().await {
        if conn_tx.write_frame(m).await.is_err() {
            break;
        }
    }
}

struct Client {
    state: ServerState,
    conn_rx: ConnectionRx,
    tx: mpsc::Sender<Message>,
    player_id: PlayerId,
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
        }
    }

    /// Takes ownership of self to guarantee that client will be dropped when it's
    /// message loop ends
    pub async fn run(mut self) {
        let lobby_handle = self.handshake().await.unwrap();
        let mut handler = MessageHandler::new(lobby_handle);
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
            match handler.process(incoming).await {
                Ok(()) => (),
                Err(e) => {
                    log::error!("Player {:#X} encountered error {e:?}", self.player_id);
                    let _ = self
                        .tx
                        .send(Message::Error {
                            error: ProtocolError::Message(e.to_string()),
                        })
                        .await;
                }
            }
        }
        log::info!("Player {:#X} disconnected", self.player_id);
    }

    async fn handshake(&mut self) -> Result<LobbyHandle, ProtocolError> {
        let version = match self.conn_rx.read_frame().await.unwrap() {
            Some(Message::Version { version }) => version,
            Some(_) => return Err(ProtocolError::InvalidMessage),
            None => return Err(ProtocolError::Disconnected),
        };

        if version != crate::VERSION {
            return Err(ProtocolError::VersionMismatch(version, crate::VERSION.to_owned()).into());
        }

        // Inform player of their PlayerId
        self.tx
            .send(Message::ConnectionAccept {
                player_id: self.player_id,
            })
            .await
            .unwrap();
        log::info!("New connection for player id {:#X} opened", self.player_id);

        let lobby_handle = match self.conn_rx.read_frame().await.unwrap() {
            Some(Message::GameHost) => {
                let state = &mut *self.state.lock().unwrap();
                state.players.insert(self.player_id);
                state
                    .add_lobby(self.state.clone())
                    .get_handle(self.player_id)
            }
            Some(Message::GameJoin { lobby_id }) => {
                let state = &mut *self.state.lock().unwrap();
                state.players.insert(self.player_id);
                state
                    .lobbies
                    .get_mut(&lobby_id)
                    .ok_or(ProtocolError::InvalidLobbyId(lobby_id))?
                    .get_handle(self.player_id)
            }
            Some(_) => return Err(ProtocolError::InvalidMessage),
            None => return Err(ProtocolError::Disconnected),
        };

        let lobby_recv = lobby_handle.join_lobby().await.unwrap();
        tokio::spawn(broadcast_task(lobby_recv, self.tx.clone()));
        return Ok(lobby_handle);
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // This will crash the program if we're dropping due to a previous panic caused by a poisoned lock,
        // and that's fine for now.
        let state = &mut *self.state.lock().unwrap();
        state.players.remove(&self.player_id);
    }
}

// TODO: Separate handlers for host/non-host clients. May be difficult to deal with transferring host to another player
struct MessageHandler {
    lobby_handle: ManuallyDrop<LobbyHandle>,
}

impl MessageHandler {
    fn new(lobby_handle: LobbyHandle) -> Self {
        Self {
            lobby_handle: ManuallyDrop::new(lobby_handle),
        }
    }

    async fn process(&mut self, msg: Message) -> Result<(), LobbyError> {
        match msg {
            Message::GameBegin => {
                self.lobby_handle.start_game().await?;
            }
            Message::PlayerOptions { options } => {
                self.lobby_handle.set_player_options(options).await?;
            }
            Message::PlayerCanStart(val) => {
                self.lobby_handle.set_player_can_start(val).await?;
            }
            Message::GameOptions { options } => {
                self.lobby_handle.set_game_options(options).await?;
            }
            Message::GameCurrentLevel { level } => {
                self.lobby_handle.set_player_level(level).await?;
            }
            Message::GameItemCollected { item } => {
                self.lobby_handle.player_collected_item(item).await?;
            }
            _ => {
                // return Err(ProtocolError::InvalidMessage)
                //     .context("Client sent a server-only message.")
                // TODO: return error
                todo!("Client sent a server-only message");
            }
        }

        Ok(())
    }
}

impl Drop for MessageHandler {
    fn drop(&mut self) {
        // SAFETY: ManuallyDrop::take requires us to never use the original container again.
        // Since we are currently dropping ourself, self.lobby_handle will never be used again.
        let lobby_handle = unsafe { ManuallyDrop::take(&mut self.lobby_handle) };
        tokio::spawn(async move { lobby_handle.rem_player().await });
    }
}
