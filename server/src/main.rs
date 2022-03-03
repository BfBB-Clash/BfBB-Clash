use clash::lobby::LobbyOptions;
use clash::player::{PlayerOptions, SharedPlayer};
use clash::protocol::{Connection, Item, Message};
use log::{debug, error, info, warn};
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::{select, spawn};

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
    pub fn add_player(&mut self) -> u32 {
        let auth_id = self.gen_auth_id();
        self.players.insert(
            auth_id,
            Player::new(
                SharedPlayer::new(PlayerOptions {
                    name: String::new(),
                    color: 0,
                }),
                auth_id,
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
    #[allow(dead_code)]
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
        .filter_level(log::LevelFilter::Debug)
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
        spawn(handle_new_connection(state, socket));
    }
}

async fn handle_new_connection(state: Arc<RwLock<State>>, mut socket: TcpStream) {
    // Add new player
    let auth_id = {
        let mut state = state.write().expect("Failed to lock State");
        state.add_player()
    };
    let mut connection = Connection::new(&mut socket);

    // Inform player of their auth_id
    if let Err(e) = connection
        .write_frame(Message::ConnectionAccept { auth_id })
        .await
    {
        error!("Couldn't communicate with new player {auth_id:#X}.\n{e:?}");
        return;
    }
    info!("New connection for player id {auth_id:#X} opened");

    let (mut lobby_send, mut lobby_recv): (Option<Sender<Message>>, Option<Receiver<Message>>) =
        (None, None);
    loop {
        select! {
            m = async { lobby_recv.as_mut().unwrap().recv().await }, if lobby_recv.is_some() => {
                connection.write_frame(m.unwrap()).await.unwrap();
            }
            incoming = connection.read_frame() => {
                let incoming = match incoming {
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
                debug!("Received message from player id {auth_id:#X} \nMessage: {incoming:#X?}",);
                if !process_incoming(state.clone(), auth_id, incoming, &mut lobby_send, &mut lobby_recv).await {
                    info!("Disconnecting player {auth_id:#X} due to unrecoverable error.");
                    break;
                }
            }
        };
    }

    // Clean up player
    let mut state = state.write().unwrap();
    state.players.remove(&auth_id);
}

async fn process_incoming(
    state: Arc<RwLock<State>>,
    auth_id: u32,
    incoming: Message,
    lobby_send: &mut Option<Sender<Message>>,
    lobby_recv: &mut Option<Receiver<Message>>,
) -> bool {
    match incoming {
        Message::GameHost {
            auth_id: _,
            lobby_id: _,
        } => {
            let state = &mut *state.write().unwrap();
            if state.players.contains_key(&auth_id) {
                let gen_lobby_id = state.gen_lobby_id();
                state.lobbies.insert(
                    gen_lobby_id,
                    Lobby::new(gen_lobby_id, LobbyOptions::default(), gen_lobby_id, auth_id),
                );

                let lobby = match state.lobbies.get_mut(&gen_lobby_id) {
                    None => {
                        error!("Attempted to join lobby with an invalid id '{gen_lobby_id}'");
                        return true;
                    }
                    Some(l) => l,
                };

                let tmp = lobby.add_player(&mut state.players, auth_id).unwrap();
                *lobby_send = Some(tmp.0);
                *lobby_recv = Some(tmp.1);

                info!("Player {auth_id:#X} has hosted lobby {gen_lobby_id:#X}");
                lobby_send
                    .as_mut()
                    .unwrap()
                    .send(Message::GameLobbyInfo {
                        auth_id: 0,
                        lobby: lobby.shared.clone(),
                    })
                    .unwrap();
            }
        }
        Message::GameJoin {
            auth_id: _,
            lobby_id,
        } => {
            let state = &mut *state.write().unwrap();

            if state.players.contains_key(&auth_id) {
                let lobby = match state.lobbies.get_mut(&lobby_id) {
                    None => {
                        error!("Attempted to join lobby with an invalid id '{lobby_id}'");
                        return true;
                    }
                    Some(l) => l,
                };

                // TODO: Waiting for destructuring assignment
                let tmp = lobby.add_player(&mut state.players, auth_id).unwrap();
                *lobby_send = Some(tmp.0);
                *lobby_recv = Some(tmp.1);

                info!("Player {auth_id:#X} has joined lobby {lobby_id:#X}");
                lobby_send
                    .as_mut()
                    .unwrap()
                    .send(Message::GameLobbyInfo {
                        auth_id: 0,
                        lobby: lobby.shared.clone(),
                    })
                    .unwrap();
            } else {
                error!("Player {auth_id:#X} not in the playerlist");
            }
        }
        Message::GameLobbyInfo {
            auth_id: _,
            lobby: _,
        } => todo!(),
        Message::GameBegin { auth_id: _ } => todo!(),
        Message::GameEnd { auth_id: _ } => todo!(),
        Message::GameLeave { auth_id: _ } => {
            let state = &mut *state.write().unwrap();

            let lobby_id = match state.players.get_mut(&auth_id) {
                Some(p) => p.lobby_id,
                None => {
                    error!("Invalid player id {auth_id:#X}");
                    //TODO: Kick player?
                    return false;
                }
            };

            let lobby = match state.lobbies.get_mut(&lobby_id) {
                None => {
                    error!("Attempted to leave lobby with an invalid id '{lobby_id}'");
                    return true;
                }
                Some(l) => l,
            };

            if lobby.is_player_in_lobby(&auth_id) {
                lobby.rem_player(&mut state.players, auth_id);
            }
        }
        Message::PlayerOptions { auth_id, options } => {
            let state = &mut *state.write().unwrap();

            let player = match state.players.get_mut(&auth_id) {
                Some(p) => {
                    p.shared.options = options.clone();
                    p
                }
                None => {
                    error!("Invalid player id {auth_id:#X}");
                    //TODO: Kick player?
                    return false;
                }
            };
            match state.lobbies.get_mut(&player.lobby_id) {
                Some(l) => {
                    if let Some(index) = player.shared.lobby_index {
                        if index < l.shared.players.len() {
                            l.shared.players[index] = player.shared.clone();
                        }
                    }
                }
                None => {
                    error!("Invalid lobby id {:#X}", player.lobby_id);
                }
            }

            if let Some(lobby_send) = lobby_send.as_mut() {
                let message = Message::PlayerOptions {
                    auth_id: 0,
                    options,
                };
                lobby_send.send(message).unwrap();
            }
        }
        Message::GameOptions { auth_id, options } => {
            let state = &mut *state.write().unwrap();
            let lobby_id = match state.players.get_mut(&auth_id) {
                Some(p) => p.shared.current_lobby,
                None => {
                    error!("Invalid player id {auth_id:#X}");
                    //TODO: Ditto
                    return false;
                }
            };

            if let Some(lobby_send) = lobby_send {
                match state.lobbies.get_mut(&lobby_id) {
                    Some(l) => {
                        if l.host_id == auth_id {
                            l.shared.options = options.clone();
                            let message = Message::GameOptions {
                                auth_id: 0,
                                options,
                            };
                            lobby_send.send(message).unwrap();
                        }
                    }
                    None => {
                        error!("Invalid lobby id {lobby_id:#X}");
                    }
                }
            }
        }
        Message::GameCurrentRoom { auth_id: _, room } => {
            let state = &mut *state.write().unwrap();

            let lobby_id = match state.players.get_mut(&auth_id) {
                Some(p) => p.lobby_id,
                None => {
                    error!("Invalid player id {auth_id:#X}");
                    return false;
                }
            };

            let lobby = match state.lobbies.get_mut(&lobby_id) {
                Some(l) => {
                    l.shared.game_state.current_room = room;
                    info!("Player {auth_id:#X} entered {room:?}");
                    l.shared.clone()
                }
                None => {
                    error!("Invalid lobby id {lobby_id:#X}");
                    return false;
                }
            };

            if let Some(lobby_send) = lobby_send {
                lobby_send
                    .send(Message::GameLobbyInfo { auth_id: 0, lobby })
                    .unwrap();
            }
        }
        Message::GameItemCollected { auth_id: _, item } => {
            let state = &mut *state.write().unwrap();

            let (lobby_id, index) = match state.players.get_mut(&auth_id) {
                Some(p) => (p.lobby_id, p.shared.lobby_index),
                None => {
                    error!("Invalid player id {auth_id:#X}");
                    return false;
                }
            };

            let lobby = match state.lobbies.get_mut(&lobby_id) {
                Some(l) => {
                    if let Item::Spatula(spat) = item {
                        l.shared.game_state.spatulas.insert(spat, index);
                        info!("Player {auth_id:#X} collected {spat:?}");
                    }
                    l.shared.clone()
                }
                None => {
                    error!("Invalid lobby id {lobby_id:#X}");
                    return false;
                }
            };

            if let Some(lobby_send) = lobby_send {
                lobby_send
                    .send(Message::GameLobbyInfo { auth_id: 0, lobby })
                    .unwrap();
            }
        }
        m => {
            warn!("Player id {auth_id:#X} sent a server only message. \nMessage: {m:?}");
        }
    }
    true
}
