use std::sync::mpsc::Sender;

use clash_lib::net::{
    connection::{self, ConnectionRx},
    LobbyMessage, Message,
};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

pub type NetCommandReceiver = mpsc::Receiver<NetCommand>;
pub type NetCommandSender = mpsc::Sender<NetCommand>;

#[derive(Clone, Debug)]
pub enum NetCommand {
    Disconnect,
    Send(Message),
}

impl<T> From<T> for NetCommand
where
    Message: From<T>,
{
    fn from(msg: T) -> Self {
        Self::Send(msg.into())
    }
}

#[tokio::main]
pub async fn run(
    mut receiver: NetCommandReceiver,
    logic_sender: Sender<Message>,
    error_sender: Sender<anyhow::Error>,
) {
    let ip = load_ip_address();
    tracing::info!("Connecting to server at '{ip}'");

    let sock = TcpStream::connect(&ip).await.unwrap();
    let (mut conn_tx, conn_rx) = connection::from_socket(sock);
    conn_tx
        .write_frame(Message::Version {
            version: crate::VERSION.to_owned(),
        })
        .await
        .unwrap();

    let recv_task = tokio::spawn(recv_task(conn_rx, error_sender.clone(), logic_sender));
    while let Some(command) = receiver.recv().await {
        // NetCommand should be a Disconnect or Send command
        let msg = match command {
            NetCommand::Disconnect => break,
            NetCommand::Send(m) => m,
        };
        tracing::debug!("Sending message {msg:#?}");
        if let Err(e) = conn_tx.write_frame(msg).await {
            tracing::error!("Error sending message to server. Disconnecting. {e:#?}");
            error_sender
                .send(e.into())
                .expect("GUI has crashed and so will we.");
            break;
        }
    }
    recv_task.abort();
    tracing::info!("Disconnected from server.")
}

async fn recv_task(
    mut conn_rx: ConnectionRx,
    error_sender: Sender<anyhow::Error>,
    logic_sender: Sender<Message>,
) {
    loop {
        let incoming = match conn_rx.read_frame().await {
            Ok(Some(x)) => {
                tracing::debug!("Received message {x:#?}.");
                x
            }
            Ok(None) => {
                tracing::info!("Server closed connection. Disconnecting.");
                break;
            }
            Err(e) => {
                tracing::error!("Error reading message from server. Disconnecting.\n{e}");
                error_sender
                    .send(e.into())
                    .expect("GUI has crashed and so will we.");
                break;
            }
        };

        match incoming {
            Message::Lobby(act) => process_action(act, &logic_sender),
            m @ Message::ConnectionAccept { player_id: _ } => {
                tracing::debug!("ConnectionAccept message got :)");
                logic_sender.send(m).unwrap();
                continue;
            }
            m @ Message::GameLobbyInfo { lobby: _ } => {
                logic_sender.send(m).unwrap();
                continue;
            }
            Message::Error { error } => {
                tracing::error!("Error from server:\n{error}");
                error_sender
                    .send(error.into())
                    .expect("GUI has crashed and so will we.");
                continue;
            }
            _ => {
                tracing::error!("Invalid message received from server");
                continue;
            }
        }
    }
}

fn process_action(action: LobbyMessage, logic_sender: &Sender<Message>) {
    match action {
        m @ LobbyMessage::GameBegin => {
            logic_sender.send(Message::Lobby(m)).unwrap();
        }
        LobbyMessage::GameEnd => {
            // This message isn't supposed to do anything until the GUI gets updated.
        }
        // We aren't yet doing partial updates
        LobbyMessage::ResetLobby => todo!(),
        LobbyMessage::PlayerOptions { options: _ } => todo!(),
        LobbyMessage::PlayerCanStart(_) => todo!(),
        LobbyMessage::GameOptions { options: _ } => todo!(),
        LobbyMessage::GameCurrentLevel { level: _ } => todo!(),
        LobbyMessage::GameItemCollected { item: _ } => todo!(),
    }
}

fn load_ip_address() -> String {
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        exe_path.push("ipaddress");
        if let Ok(ip) = std::fs::read_to_string(exe_path) {
            return ip.trim().to_string();
        }
    }

    "127.0.0.1:42932".into()
}
