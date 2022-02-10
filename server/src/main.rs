use clash::protocol::{Connection, Message};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;

pub mod lobby;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let listener = TcpListener::bind("127.0.0.1:42932").await.unwrap();

    loop {
        let (socket, port) = listener.accept().await.unwrap();
        println!("{socket:?} \n{port:?}");

        spawn(async move { handle_connection(socket).await });
    }
}

async fn handle_connection(socket: TcpStream) {
    let mut connection = Connection::new(socket);

    // Generate an auth id for this user
    // TODO
    let auth_id = 1;
    connection
        .write_frame(Message::ConnectionAccept { auth_id })
        .await;

    loop {
        let incoming = connection.read_frame().await;

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
                println!("Client sent a server only message: {m:?}")
            }
        }
    }
}
