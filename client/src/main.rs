use clash::protocol::Connection;
use tokio::net::TcpStream;

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
        .filter_level(log::LevelFilter::Warn)
        .parse_env("CLASH_LOG")
        .init();

    // Create a new thread and start a tokio runtime on it for talking to the server
    // TODO: Tokio may not be the best tool for the client. It might be better to
    //       simply use std's blocking networking in a new thread, since we should only ever
    //       have a single connection. Unfortunately for now we need to use it since the shared
    //       library is async.
    let _network_thread = std::thread::Builder::new()
        .name("Network".into())
        .spawn(start_network);

    // Start Game Thread
    let _game_thread = std::thread::Builder::new()
        .name("Logic".into())
        .spawn(game::start_game);

    // Start gui on the main thread
    gui::run();
}

#[tokio::main(flavor = "current_thread")]
async fn start_network() {
    let sock = TcpStream::connect("127.0.0.1:42932").await.unwrap();
    let mut conn = Connection::new(sock);

    conn.write_frame(clash::protocol::Message::GameHost {
        auth_id: 1,
        lobby_id: 2,
    })
    .await
    .unwrap();
}
