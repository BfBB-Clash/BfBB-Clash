mod client;
mod lobby;
mod state;

use state::ServerState;
use tokio::net::TcpListener;
use tracing::metadata::LevelFilter;

const VERSION: &str = env!("CLASH_VERSION");
const DEFAULT_PORT: u16 = 42932;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .init();

    tracing::info!("Server Version: {}", crate::VERSION);

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let listener = TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    tracing::info!("Listening on port {port}");

    let state = ServerState::default();
    loop {
        let (socket, _) = listener.accept().await.unwrap();

        tokio::spawn(client::handle_new_connection(state.clone(), socket));
    }
}
