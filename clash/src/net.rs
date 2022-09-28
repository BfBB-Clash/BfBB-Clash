use std::sync::mpsc::Sender;

use async_scoped::TokioScope;
use clash_lib::net::{
    connection::{self, ConnectionRx},
    LobbyMessage, Message,
};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

pub type NetCommandSender = mpsc::Sender<NetCommand>;
pub type ErrorReceiver = std::sync::mpsc::Receiver<anyhow::Error>;

#[derive(Clone, Debug)]
pub enum NetCommand {
    Connect,
    Disconnect,
    Send(Message),
}

#[tokio::main]
pub async fn run(
    mut connect_recv: mpsc::Receiver<NetCommand>,
    logic_sender: Sender<Message>,
    error_sender: Sender<anyhow::Error>,
) {
    while let Some(comm) = connect_recv.recv().await {
        match comm {
            NetCommand::Connect => {
                TokioScope::scope_and_block(|s| {
                    s.spawn(main_task(
                        &mut connect_recv,
                        logic_sender.clone(),
                        error_sender.clone(),
                    ));
                });
                log::info!("Disconnected from server.");
            }
            // We may receieve a disconnect command here if the server connection is lost and the client
            // then clicks the "Leave Lobby" button.
            NetCommand::Disconnect => continue,
            NetCommand::Send(_) => {
                log::error!("Attempted to send message while not connected to server.");
            }
        }
    }
}

async fn main_task(
    receiver: &mut mpsc::Receiver<NetCommand>,
    logic_sender: Sender<Message>,
    error_sender: Sender<anyhow::Error>,
) {
    let ip = load_ip_address();
    log::info!("Connecting to server at '{ip}'");

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
            NetCommand::Connect => {
                log::error!("Attempted to connect to server while already connected");
                continue;
            }
            NetCommand::Disconnect => break,
            NetCommand::Send(m) => m,
        };
        log::debug!("Sending message {msg:#?}");
        if let Err(e) = conn_tx.write_frame(msg).await {
            log::error!("Error sending message to server. Disconnecting. {e:#?}");
            error_sender
                .send(e.into())
                .expect("GUI has crashed and so will we.");
            break;
        }
    }
    recv_task.abort();
}

async fn recv_task(
    mut conn_rx: ConnectionRx,
    error_sender: Sender<anyhow::Error>,
    logic_sender: Sender<Message>,
) {
    loop {
        let incoming = match conn_rx.read_frame().await {
            Ok(Some(x)) => {
                log::debug!("Received message {x:#?}.");
                x
            }
            Ok(None) => {
                log::info!("Server closed connection. Disconnecting.");
                break;
            }
            Err(e) => {
                log::error!("Error reading message from server. Disconnecting.\n{e}");
                error_sender
                    .send(e.into())
                    .expect("GUI has crashed and so will we.");
                break;
            }
        };

        match incoming {
            Message::Lobby(act) => process_action(act, &logic_sender),
            m @ Message::ConnectionAccept { player_id: _ } => {
                log::debug!("ConnectionAccept message got :)");
                logic_sender.send(m).unwrap();
                continue;
            }
            Message::Error { error } => {
                log::error!("Error from server:\n{error}");
                error_sender
                    .send(error.into())
                    .expect("GUI has crashed and so will we.");
                continue;
            }
            _ => {
                log::error!("Invalid message received from server");
                continue;
            }
        }
    }
}

fn process_action(action: LobbyMessage, logic_sender: &Sender<Message>) {
    match action {
        m @ LobbyMessage::GameLobbyInfo { lobby: _ } => {
            logic_sender.send(Message::Lobby(m)).unwrap();
        }
        m @ LobbyMessage::GameBegin => {
            logic_sender.send(Message::Lobby(m)).unwrap();
        }
        LobbyMessage::GameEnd => {
            // This message isn't supposed to do anything until the GUI gets updated.
        }
        // We aren't yet doing partial
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
