use std::mem::ManuallyDrop;

use clash_lib::net::connection::{self, ConnectionRx, ConnectionTx};
use clash_lib::net::{LobbyMessage, Message, ProtocolError};
use clash_lib::PlayerId;
use tokio::net::TcpStream;
use tokio::select;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tracing::instrument;

use crate::lobby::lobby_handle::LobbyHandle;
use crate::lobby::LobbyError;
use crate::state::ServerState;

/// Take a socket for a newly connected client and begin serving it.
pub async fn handle_new_connection(state: ServerState, socket: TcpStream) {
    let client = match ConnectingClient::new(state, socket).handshake().await {
        Some(c) => c,
        None => return,
    };
    client.run().await;
}

/// Represents a client who just connected and still needs to tell the server what they want to do.
struct ConnectingClient {
    state: ServerState,
    player_id: PlayerId,
    conn_tx: ConnectionTx,
    conn_rx: ConnectionRx,
}

impl ConnectingClient {
    fn new(state: ServerState, socket: TcpStream) -> Self {
        // Add new player
        let player_id = {
            let mut state = state.lock().unwrap();
            state.add_player()
        };

        let (conn_tx, conn_rx) = connection::from_socket(socket);
        Self {
            state,
            player_id,
            conn_tx,
            conn_rx,
        }
    }

    async fn handshake(mut self) -> Option<ConnectedClient> {
        match self.try_handshake().await {
            Ok(it) => Some(it.construct(self)),
            Err(error) => {
                tracing::error!(%error);
                let _ = self.conn_tx.write_frame(Message::Error { error }).await;
                None
            }
        }
    }

    async fn try_handshake(&mut self) -> Result<ClientConstructor, ProtocolError> {
        let version = match self.conn_rx.read_frame().await? {
            Some(Message::Version { version }) => version,
            Some(_) => return Err(ProtocolError::InvalidMessage),
            None => return Err(ProtocolError::Disconnected),
        };

        if version != crate::VERSION {
            return Err(ProtocolError::VersionMismatch(
                version,
                crate::VERSION.to_owned(),
            ));
        }

        // Inform player of their PlayerId
        self.conn_tx
            .write_frame(Message::ConnectionAccept {
                player_id: self.player_id,
            })
            .await?;
        tracing::info!("New connection for player id {} opened", self.player_id);

        let lobby_handle = match self.conn_rx.read_frame().await? {
            Some(Message::GameHost) => {
                let state = &mut *self.state.lock().unwrap();
                state.players.insert(self.player_id);
                state.open_lobby(self.state.clone(), self.player_id)
            }
            Some(Message::GameJoin { lobby_id, spectate }) => {
                let handle_provider = {
                    let state = &mut *self.state.lock().unwrap();
                    state.players.insert(self.player_id);
                    state
                        .lobbies
                        .get_mut(&lobby_id)
                        .ok_or(ProtocolError::InvalidLobbyId(lobby_id))?
                        .clone()
                };

                if spectate {
                    let recv = handle_provider.spectate().await?;
                    return Ok(ClientConstructor::Spectator(recv));
                } else {
                    handle_provider.into_handle(self.player_id)?
                }
            }
            Some(_) => return Err(ProtocolError::InvalidMessage),
            None => return Err(ProtocolError::Disconnected),
        };

        let lobby_recv = lobby_handle
            .join_lobby()
            .await
            .map_err(|err| ProtocolError::Message(err.to_string()))?;
        Ok(ClientConstructor::Player(lobby_handle, lobby_recv))
    }
}

/// Our [`ConnectedClient`] constructors need to take ownership of a [`ConnectedClient`].
/// This allows us to return what kind of client to construct from `try_handshake` to the caller,
/// since the caller needs to retain ownership of `self` for error reporting to the client.
enum ClientConstructor {
    Player(LobbyHandle, broadcast::Receiver<Message>),
    Spectator(broadcast::Receiver<Message>),
}

impl ClientConstructor {
    fn construct(self, client: ConnectingClient) -> ConnectedClient {
        match self {
            ClientConstructor::Player(lobby_handle, lobby_recv) => {
                PlayerClient::from_connecting(client, lobby_handle, lobby_recv).into()
            }
            ClientConstructor::Spectator(lobby_recv) => {
                SpectatingClient::from_connecting(client, lobby_recv).into()
            }
        }
    }
}

enum ConnectedClient {
    Player(PlayerClient),
    Spectator(SpectatingClient),
}

impl From<PlayerClient> for ConnectedClient {
    fn from(val: PlayerClient) -> Self {
        ConnectedClient::Player(val)
    }
}

impl From<SpectatingClient> for ConnectedClient {
    fn from(val: SpectatingClient) -> Self {
        ConnectedClient::Spectator(val)
    }
}

impl ConnectedClient {
    async fn run(self) {
        match self {
            ConnectedClient::Player(c) => c.run().await,
            ConnectedClient::Spectator(c) => c.run().await,
        }
    }
}

async fn send_task(
    mut conn_tx: ConnectionTx,
    mut lobby_rx: tokio::sync::broadcast::Receiver<Message>,
    mut local_rx: tokio::sync::mpsc::Receiver<Message>,
) {
    loop {
        let m = select! {
            Ok(m) = lobby_rx.recv() => m,
            Some(m) = local_rx.recv() => m,
            else => return,
        };

        if conn_tx.write_frame(m).await.is_err() {
            return;
        }
    }
}

/// Used to represent a client who is in a lobby.
struct PlayerClient {
    state: ServerState,
    player_id: PlayerId,
    conn_rx: ConnectionRx,
    local_tx: mpsc::Sender<Message>,
    task_handle: JoinHandle<()>,
    lobby_handle: ManuallyDrop<LobbyHandle>,
}

impl PlayerClient {
    pub fn from_connecting(
        client: ConnectingClient,
        lobby_handle: LobbyHandle,
        lobby_recv: broadcast::Receiver<Message>,
    ) -> Self {
        let lobby_handle = ManuallyDrop::new(lobby_handle);
        let (tx, rx) = mpsc::channel(64);
        let task_handle = tokio::spawn(send_task(client.conn_tx, lobby_recv, rx));

        PlayerClient {
            state: client.state,
            player_id: client.player_id,
            conn_rx: client.conn_rx,
            local_tx: tx,
            task_handle,
            lobby_handle,
        }
    }

    /// Takes ownership of self to guarantee that client will be dropped when it's
    /// message loop ends
    #[instrument(skip_all, fields(player_id = %self.player_id))]
    pub async fn run(mut self) {
        loop {
            let incoming = match self.conn_rx.read_frame().await {
                Ok(Some(Message::Lobby(x))) => x,
                Ok(Some(m)) => {
                    tracing::error!("Invalid message received: {m:?}");
                    let _ = self
                        .local_tx
                        .send(Message::Error {
                            error: ProtocolError::InvalidMessage,
                        })
                        .await;
                    continue;
                }
                Ok(None) => {
                    break;
                }
                Err(e) => {
                    tracing::error!("Error reading message, Closing connection\n{e:?}",);
                    break;
                }
            };

            tracing::debug!("Received message: {incoming:#?}");
            match self.process(incoming).await {
                Ok(()) => (),
                Err(e) => {
                    tracing::error!("Encountered error processing message: {e:?}");
                    let _ = self
                        .local_tx
                        .send(Message::Error {
                            error: ProtocolError::Message(e.to_string()),
                        })
                        .await;
                }
            }
        }
        tracing::info!("Player disconnected");
    }

    async fn process(&mut self, msg: LobbyMessage) -> Result<(), LobbyError> {
        match msg {
            LobbyMessage::PlayerOptions { options } => {
                self.lobby_handle.set_player_options(options).await
            }
            LobbyMessage::PlayerCanStart(val) => self.lobby_handle.set_player_can_start(val).await,
            LobbyMessage::ResetLobby => self.lobby_handle.reset_lobby().await,
            LobbyMessage::GameOptions { options } => {
                self.lobby_handle.set_game_options(options).await
            }
            LobbyMessage::GameBegin => self.lobby_handle.start_game().await,
            LobbyMessage::GameCurrentLevel { level } => {
                self.lobby_handle.set_player_level(level).await
            }
            LobbyMessage::GameItemCollected { item } => {
                self.lobby_handle.player_collected_item(item).await
            }
            LobbyMessage::GameEnd => todo!(),
        }
    }
}

impl Drop for PlayerClient {
    fn drop(&mut self) {
        self.task_handle.abort();

        // SAFETY: ManuallyDrop::take requires us to never use the ManuallyDrop container again.
        // Since we are currently dropping ourself, self.lobby_handle will never be used again.
        let lobby_handle = unsafe { ManuallyDrop::take(&mut self.lobby_handle) };
        tokio::spawn(async move { lobby_handle.rem_player().await });

        // This will crash the program if we're dropping due to a previous panic caused by a poisoned lock,
        // and that's fine for now.
        let state = &mut *self.state.lock().unwrap();
        state.players.remove(&self.player_id);
    }
}

// TODO: Abstract client types and deduplicate code.
struct SpectatingClient {
    state: ServerState,
    player_id: PlayerId,
    conn_rx: ConnectionRx,
    local_tx: mpsc::Sender<Message>,
    task_handle: JoinHandle<()>,
}

impl SpectatingClient {
    pub fn from_connecting(
        client: ConnectingClient,
        lobby_recv: broadcast::Receiver<Message>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(64);
        let task_handle = tokio::spawn(send_task(client.conn_tx, lobby_recv, rx));

        Self {
            state: client.state,
            player_id: client.player_id,
            conn_rx: client.conn_rx,
            local_tx: tx,
            task_handle,
        }
    }

    /// Takes ownership of self to guarantee that client will be dropped when it's
    /// message loop ends
    #[instrument(skip_all, fields(player_id = %self.player_id))]
    pub async fn run(mut self) {
        loop {
            match self.conn_rx.read_frame().await {
                // Spectators should never send a message again after joining
                Ok(Some(m)) => {
                    tracing::error!("Invalid message received: {m:?}");
                    let _ = self
                        .local_tx
                        .send(Message::Error {
                            error: ProtocolError::InvalidMessage,
                        })
                        .await;
                    continue;
                }
                Ok(None) => {
                    break;
                }
                Err(e) => {
                    tracing::error!("Error reading message, Closing connection\n{e:?}",);
                    break;
                }
            };
        }
        tracing::info!("Player disconnected");
    }
}

impl Drop for SpectatingClient {
    fn drop(&mut self) {
        self.task_handle.abort();

        // This will crash the program if we're dropping due to a previous panic caused by a poisoned lock,
        // and that's fine for now.
        let state = &mut *self.state.lock().unwrap();
        state.players.remove(&self.player_id);
    }
}
