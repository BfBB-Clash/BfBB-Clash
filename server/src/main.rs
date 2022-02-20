use clash::player::{PlayerOptions, SharedPlayer};
use clash::protocol::{self, Connection, Message};
use log::{debug, error, info, warn};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::sync::{self, Arc, Mutex, RwLock, mpsc::Sender};
use tokio::net::{TcpListener, TcpStream};
use tokio::spawn;

pub mod lobby;
pub mod player;

use crate::lobby::Lobby;
use crate::player::Player;

pub struct State {
    players: HashMap<u32, Player>,
    lobbies: HashMap<u32, Lobby>,
}

impl State {
    fn new() -> Self {
        Self {
            players: HashMap::new(),
            lobbies: HashMap::new(),
        }
    }
    pub fn add_player(&mut self, send: Mutex<Sender<Message>>) -> u32 {
        let auth_id = self.gen_auth_id();
        self.players.insert(
            auth_id,
            Player::new(
                SharedPlayer::new(PlayerOptions {
                    name: String::new(),
                    color: 0,
                }),
                auth_id,
                send
            ),
        );
        auth_id
    }
    // TODO: dedupe this.
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
    fn gen_lobby_id(&self) -> u32 {
        let mut lobby_id;
        loop {
            lobby_id = thread_rng().gen();
            if !self.lobbies.contains_key(&lobby_id) {
                break;
            };
        }
        lobby_id
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
        create_new_player(state, socket);
    }
}

fn create_new_player(state: Arc<RwLock<State>>, mut socket: TcpStream) {
    let (sender, receiver) = std::sync::mpsc::channel::<Message>();
    let sender = Mutex::new(sender);
    let auth_id = {
        let mut state = state.write().expect("Failed to lock State");
        state.add_player(sender)
    };
    spawn(async move { handle_new_connection(state, socket, auth_id, receiver).await });
}

async fn handle_new_connection(state: Arc<RwLock<State>>, mut socket: TcpStream, auth_id: u32, receiver: std::sync::mpsc::Receiver<Message>) {
    // Add new player
    let mut connection = Connection::new(&mut socket);
    // Inform player of their auth_id
    if let Err(e) = connection
        .write_frame(Message::ConnectionAccept { auth_id })
        .await
    {
        error!("Couldn't communicate with new player {auth_id:#X}.");
        return;
    }
    info!("New connection for player id {auth_id:#X} opened");
    let mut send_message_buf = vec![];
    loop {
        loop {
            match receiver.try_recv() {
                Ok(m) => {
                    send_message_buf.push(m);
                }
                Err(e) => {
                    break;
                }
            }
        }

        for m in send_message_buf.drain(..) {
            let _ = connection.write_frame(m).await;
        }

        let incoming = match connection.read_frame() {
            Ok(Some(x)) => x,
            Ok(None) => {
                info!("Player id {auth_id:#X} disconnected");
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                error!(
                    "Error reading message from player id {auth_id:#X}. Closing connection\n{e:?}"
                );
                break;
            }
        };

        debug!("Received message from player id {auth_id:#X} \nMessage: {incoming:#?}",);
        {
            match incoming {
                Message::GameHost { auth_id, lobby_id } => todo!(),
                Message::GameJoin { auth_id, lobby_id } => todo!(),
                Message::GameLobbyInfo { auth_id, lobby } => todo!(),
                Message::GameBegin { auth_id } => todo!(),
                Message::GameEnd { auth_id } => todo!(),
                Message::GameLeave { auth_id } => todo!(),
                Message::PlayerOptions { auth_id, options } => {
                    let state = &mut *state.write().unwrap();

                    let player = match state.players.get_mut(&auth_id) {
                        Some(p) => {
                            p.shared.options = options.clone();
                            p
                        }
                        None => {
                            info!("Invalid player id {auth_id:#X}");
                            //TODO: Kick player?
                            continue;
                        }
                    };
                    /*
                    match state.lobbies.get_mut(&player.lobby_id) {
                        Some(l) => {
                            if player.shared.lobby_index > -1
                                && player.shared.lobby_index < l.shared.player_count as i8
                            {
                                l.shared.players[player.shared.lobby_index as usize] =
                                    player.shared.clone();
                                let message = Message::PlayerOptions {
                                    auth_id: 0,
                                    options: options.clone(),
                                };
                                l.broadcast_message(&mut state, message);
                            }
                        }
                        None => {
                            info!("Invalid lobby id {:#X}", player.lobby_id);
                        }
                    }
                    */
                }
                Message::GameOptions { auth_id, options } => {
                    //match state.players.get_mut(&auth_id) {
                    //    Some(p) => {
                    //        let lobby_id = p.shared.current_lobby;
                    //        match state.lobbies.get_mut(&lobby_id) {
                    //            Some(l) => {
                    //                if l.host_id == auth_id {
                    //                    l.shared.options = options;
                    //                    let message = Message::GameOptions {
                    //                        auth_id: 0,
                    //                        options,
                    //                    };
                    //                    l.broadcast_message(&mut state, message);
                    //                }
                    //            }
                    //            None => {
                    //                info!("Invalid lobby id {lobby_id:#X}");
                    //            }
                    //        };
                    //    }
                    //    None => {
                    //        info!("Invalid player id {auth_id:#X}");
                    //        //TODO: Ditto
                    //        continue;
                    //    }
                    //};
                }
                Message::GameCurrentRoom { auth_id, room } => todo!(),
                Message::GameItemCollected { auth_id, item } => {
                    todo!()
                }
                m => {
                    warn!("Player id {auth_id:#X} sent a server only message. \nMessage: {m:?}")
                }
            }
        }
    }

    // Clean up player
    let mut state = state.write().unwrap();
    state.players.remove(&auth_id);
}

async fn handle_player_queue(receiver: sync::mpsc::Receiver<Message>, connection: &Connection<'_>) {
    todo!()
}
