mod client;
mod lobby;
mod state;

use client::Client;
use state::State;
use std::sync::{Arc, RwLock};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    env_logger::Builder::new()
        .format_level(true)
        .format_module_path(true)
        .format_target(false)
        .format_indent(Some(4))
        .format_timestamp_secs()
        .filter_level(log::LevelFilter::Debug)
        .parse_env("CLASH_LOG")
        .init();
    let listener = TcpListener::bind("0.0.0.0:42932").await.unwrap();
    log::info!("Listening on port 42932");
    // We will certainly want more than one lock for the server state. Likely at least for each
    // individual lobby
    let state = Arc::new(RwLock::new(State::new()));
    loop {
        let (socket, _) = listener.accept().await.unwrap();

        let state = state.clone();
        spawn(handle_new_connection(state, socket));
    }
}

async fn handle_new_connection(state: Arc<RwLock<State>>, socket: TcpStream) {
    let client = Client::new(state, socket).await.unwrap();
    client.run().await;

    // Player cleanup handled by drop impl
}
