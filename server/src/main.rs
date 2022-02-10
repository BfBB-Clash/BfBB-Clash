use clash::protocol::{Connection, Message};
use log::{debug, info, warn};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;

#[tokio::main]
async fn main() {
    env_logger::Builder::new()
        .format_level(true)
        .format_module_path(true)
        .format_target(false)
        .format_indent(Some(4))
        .format_timestamp_secs()
        .filter_level(log::LevelFilter::Warn)
        .parse_env("CLASH_LOG")
        .init();
    let listener = TcpListener::bind("127.0.0.1:42932").await.unwrap();

    loop {
        let (socket, _) = listener.accept().await.unwrap();

        spawn(async move { handle_new_connection(socket).await });
    }
}

async fn handle_new_connection(socket: TcpStream) {
    let mut connection = Connection::new(socket);

    // Generate an auth id for this user
    // TODO
    let auth_id = 1;
    connection
        .write_frame(Message::ConnectionAccept { auth_id })
        .await
        .unwrap();

    info!("New connection for player id {auth_id:#X} opened");

    loop {
        let incoming = connection.read_frame().await.unwrap();
        debug!("Received message from player id {auth_id:#X} \nMessage: {incoming:#?}",);

        match incoming {
            Message::GameHost { auth_id, lobby_id } => todo!(),
            Message::GameJoin { auth_id, lobby_id } => todo!(),
            Message::GameLobbyInfo { auth_id, lobby_id } => todo!(),
            Message::GameBegin { auth_id, lobby_id } => todo!(),
            Message::GameEnd { auth_id, lobby_id } => todo!(),
            Message::GameLeave { auth_id, lobby_id } => todo!(),
            Message::GameOptions {
                auth_id,
                lobby_id,
                options,
            } => {
                todo!()
            }
            Message::GameCurrentRoom {
                auth_id,
                lobby_id,
                room,
            } => todo!(),
            Message::GameItemCollected {
                auth_id,
                lobby_id,
                item,
            } => {
                todo!()
            }
            m => {
                warn!("Player id {auth_id:#X} sent a server only message. \nMessage: {m:?}")
            }
        }
    }
}
