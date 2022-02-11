use clash::player::Player;
use clash::protocol::{self, Connection, Message};
use log::{debug, error, info, warn};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;

pub mod lobby;
struct State {
    players: HashMap<u32, Player>,
}

impl State {
    fn new() -> Self {
        Self {
            players: HashMap::new(),
        }
    }

    fn gen_auth_id(&self) -> u32 {
        let mut auth_id;
        loop {
            auth_id = thread_rng().gen();
            if !self.players.contains_key(&auth_id) {
                break;
            };
        }
        auth_id
    }
}

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
    info!("Listening on port 42932");
    
    // We will certainly want more than one lock for the server state. Likely at least for each
    // individual lobby
    let state = Arc::new(RwLock::new(State::new()));
    loop {
        let (socket, _) = listener.accept().await.unwrap();

        let state = state.clone();
        spawn(async move { handle_new_connection(state, socket).await });
    }
}

async fn handle_new_connection(state: Arc<RwLock<State>>, socket: TcpStream) {
    let mut connection = Connection::new(socket);

    // Generate an auth id for this user
    let auth_id;
    {
        let mut state = state.write().unwrap();
        auth_id = state.gen_auth_id();
        state
            .players
            .insert(auth_id, Player::new("TODO lol".into()));
    }

    connection
        .write_frame(Message::ConnectionAccept { auth_id })
        .await
        .unwrap();
    info!("New connection for player id {auth_id:#X} opened");

    loop {
        let incoming = match connection.read_frame().await {
            Ok(Some(x)) => x,
            Ok(None) => {
                info!("Player id {auth_id:#X} disconnected");
                break;
            }
            Err(e) => {
                error!(
                    "Error reading message from player id {auth_id:#X}. Closing connection\n{e:?}"
                );
                break;
            }
        };

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

    // Clean up player
    let mut state = state.write().unwrap();
    state.players.remove(&auth_id);
}
