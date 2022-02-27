use std::sync::mpsc::{channel, Sender};

use clash::{
    player::PlayerOptions,
    protocol::{Connection, Message},
};
use log::{debug, error, info};
use tokio::{net::TcpStream, select};

mod dolphin;
mod game;
mod gui;

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
    let _game_thread = std::thread::Builder::new()
        .name("Logic".into())
        .spawn(move || game::start_game(gui_sender, network_sender, logic_receiver));

    // Start gui on the main thread
    gui::run(gui_receiver);
}

#[tokio::main(flavor = "current_thread")]
async fn start_network(
    mut receiver: tokio::sync::mpsc::Receiver<Message>,
    _logic_sender: Sender<Message>,
) {
    let mut sock = TcpStream::connect("127.0.0.1:42932").await.unwrap();
    let mut conn = Connection::new(&mut sock);

    loop {
        select! {
            m = receiver.recv() => {
                match conn.write_frame(m.unwrap()).await {
                    Ok(a) => {
                        info!("Sent message {a:#?}.")
                    }
                    Err(e) => {
                        error!("Error sending message to server. Disconnecting. {e:#?}");
                    }
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
                process_incoming(incoming, &mut conn).await;
            }
        };
    }
}

async fn process_incoming<'a>(message: Message, conn: &mut Connection<'a>) {
    match message {
        Message::ConnectionAccept { auth_id } => {
            debug!("ConnectionAccept message got :)");
            let message = Message::PlayerOptions {
                auth_id,
                options: PlayerOptions {
                    name: String::from("Will"),
                    color: 0xFFFFFFFF,
                },
            };
            match conn.write_frame(message).await {
                Ok(a) => {
                    info!("Sent message {a:#?}.")
                }
                Err(e) => {
                    error!("Error sending message to server. Disconnecting. {e:#?}");
                }
            };

            conn.write_frame(Message::GameJoin {
                auth_id,
                lobby_id: 0,
            })
            .await
            .unwrap();
        }
        Message::PlayerOptions {
            auth_id: _,
            options: _,
        } => todo!(),
        Message::GameHost {
            auth_id: _,
            lobby_id: _,
        } => todo!(),
        Message::GameJoin {
            auth_id: _,
            lobby_id: _,
        } => todo!(),
        Message::GameOptions {
            auth_id: _,
            options: _,
        } => todo!(),
        Message::GameLobbyInfo {
            auth_id: _,
            lobby: _,
        } => todo!(),
        Message::GameBegin { auth_id: _ } => todo!(),
        Message::GameCurrentRoom {
            auth_id: _,
            room: _,
        } => todo!(),
        Message::GameForceWarp {
            auth_id: _,
            room: _,
        } => todo!(),
        Message::GameItemCollected {
            auth_id: _,
            item: _,
        } => todo!(),
        Message::GameEnd { auth_id: _ } => todo!(),
        Message::GameLeave { auth_id: _ } => todo!(),
    }
}
