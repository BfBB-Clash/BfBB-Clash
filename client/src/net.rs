use std::{error::Error, sync::mpsc::Sender};

use clash_lib::net::{
    connection::{self, ConnectionTx},
    Message, ProtocolError,
};
use tokio::net::TcpStream;

#[tokio::main]
pub async fn run(
    receiver: tokio::sync::mpsc::Receiver<Message>,
    logic_sender: Sender<Message>,
    error_sender: Sender<Box<dyn Error + Send>>,
) {
    let ip = load_ip_address();
    log::info!("Connecting to server at '{ip}'");

    let sock = TcpStream::connect(&ip).await.unwrap();
    let (mut conn_tx, mut conn_rx) = connection::from_socket(sock);
    conn_tx
        .write_frame(Message::Version {
            version: crate::VERSION.to_owned(),
        })
        .await
        .unwrap();

    tokio::spawn(send_task(receiver, conn_tx, error_sender.clone()));
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

        if let Err(e) = process_incoming(incoming, &logic_sender) {
            log::error!("Error from server:\n{e}");
            error_sender
                .send(Box::new(e))
                .expect("GUI has crashed and so will we.");
        }
    }
}

async fn send_task(
    mut receiver: tokio::sync::mpsc::Receiver<Message>,
    mut tx: ConnectionTx,
    error_sender: Sender<Box<dyn Error + Send>>,
) {
    loop {
        let m = receiver.recv().await;
        log::debug!("Sending message {m:#?}");
        if let Err(e) = tx.write_frame(m.unwrap()).await {
            log::error!("Error sending message to server. Disconnecting. {e:#?}");
            error_sender
                .send(Box::new(e))
                .expect("GUI has crashed and so will we.");
        }
    }
}

fn process_incoming(message: Message, logic_sender: &Sender<Message>) -> Result<(), ProtocolError> {
    match message {
        Message::Version { version: _ } => {
            todo!()
        }
        Message::Error { error } => {
            return Err(error);
        }
        m @ Message::ConnectionAccept { player_id: _ } => {
            log::debug!("ConnectionAccept message got :)");
            logic_sender.send(m).unwrap();
        }
        Message::PlayerOptions { options: _ } => todo!(),
        Message::GameHost => todo!(),
        Message::GameJoin { lobby_id: _ } => todo!(),
        Message::GameOptions { options: _ } => todo!(),
        m @ Message::GameLobbyInfo { lobby: _ } => {
            logic_sender.send(m).unwrap();
        }
        m @ Message::GameBegin => {
            logic_sender.send(m).unwrap();
        }
        Message::GameCurrentLevel { level: _ } => todo!(),
        Message::GameForceWarp { level: _ } => todo!(),
        Message::GameItemCollected { item: _ } => todo!(),
        Message::GameEnd => {
            // This message isn't supposed to do anything until the GUI gets updated.
        }
        Message::GameLeave => todo!(),
    }

    Ok(())
}

fn load_ip_address() -> String {
    if let Ok(mut exe_path) = std::env::current_exe() {
        exe_path.pop();
        exe_path.push("ipaddress");
        if let Ok(ip) = std::fs::read_to_string(exe_path) {
            return ip;
        }
    }

    "127.0.0.1:42932".into()
}
