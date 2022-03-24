#![cfg_attr(
    all(target_os = "windows", not(feature = "console")),
    windows_subsystem = "windows"
)]

use std::sync::mpsc::{channel, Sender};

use clash::protocol::{Connection, Message};
use log::{debug, error, info};
use thiserror::Error;
use tokio::{net::TcpStream, select};

mod dolphin;
mod game;
mod gui;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    env_logger::Builder::new()
        .format_level(true)
        .format_module_path(true)
        .format_target(false)
        .format_indent(Some(4))
        .format_timestamp_secs()
        .filter_level(log::LevelFilter::Debug)
        .parse_env("CLASH_LOG")
        .init();

    let (network_sender, network_receiver) = tokio::sync::mpsc::channel::<Message>(100);
    let (logic_sender, logic_receiver) = channel::<Message>();
    // Create a new thread and start a tokio runtime on it for talking to the server
    // TODO: Tokio may not be the best tool for the client. It might be better to
    //       simply use std's blocking networking in a new thread, since we should only ever
    //       have a single connection. Unfortunately for now we need to use it since the shared
    //       library is async.
    let _network_thread = std::thread::Builder::new()
        .name("Network".into())
        .spawn(move || start_network(network_receiver, logic_sender));

    // Start Game Thread
    let (gui_sender, gui_receiver) = channel();
    let _game_thread = {
        let network_sender = network_sender.clone();
        std::thread::Builder::new()
            .name("Logic".into())
            .spawn(move || game::start_game(gui_sender, network_sender, logic_receiver))
    };

    // Start gui on the main thread
    gui::run(gui_receiver, network_sender);
}

#[derive(Error, Clone, Debug)]
enum Error {
    #[error("Client version '{VERSION}' does not match server version '{0}'")]
    VersionMismatch(String),
}

#[tokio::main(flavor = "current_thread")]
async fn start_network(
    mut receiver: tokio::sync::mpsc::Receiver<Message>,
    mut logic_sender: Sender<Message>,
) {
    let sock = TcpStream::connect("127.0.0.1:42932").await.unwrap();
    let mut conn = Connection::new(sock);
    conn.write_frame(Message::Version {
        version: env!("CARGO_PKG_VERSION").to_owned(),
    })
    .await
    .unwrap();

    loop {
        select! {
            m = receiver.recv() => {
                debug!("Sending message {m:#?}");
                if let Err(e) = conn.write_frame(m.unwrap()).await {
                    error!("Error sending message to server. Disconnecting. {e:#?}");
                };
            }
            incoming = conn.read_frame() => {
                let incoming = match incoming {
                    Ok(Some(x)) => {
                        debug!("Received message {x:#?}.");
                        x
                    }
                    Ok(None) => {
                        info!("Server closed connection. Disconnecting.");
                        break;
                    }
                    Err(e) => {
                        error!("Error reading message from server. Disconnecting.\n{e}");
                        break;
                    }
                };

                if let Err(e) = process_incoming(incoming, &mut conn, &mut logic_sender).await {
                    log::error!("{e}");
                    break;
                }
            }
        };
    }
}

async fn process_incoming(
    message: Message,
    _conn: &mut Connection,
    logic_sender: &mut Sender<Message>,
) -> Result<(), Error> {
    match message {
        Message::Version { version } => {
            // We are outdated
            return Err(Error::VersionMismatch(version));
        }
        m @ Message::ConnectionAccept { player_id: _ } => {
            debug!("ConnectionAccept message got :)");
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
        Message::GameCurrentRoom { room: _ } => todo!(),
        Message::GameForceWarp { room: _ } => todo!(),
        Message::GameItemCollected { item: _ } => todo!(),
        Message::GameEnd => todo!(),
        Message::GameLeave => todo!(),
    }

    Ok(())
}
