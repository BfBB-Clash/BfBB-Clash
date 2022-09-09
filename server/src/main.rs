mod client;
mod lobby;
mod state;

use state::ServerState;
use tokio::net::TcpListener;

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
    let state = ServerState::default();
    loop {
        let (socket, _) = listener.accept().await.unwrap();

        let state = state.clone();
        client::handle_new_connection(state, socket).await;
    }
}
