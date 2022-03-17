use clash::lobby::LobbyOptions;
use clash::protocol::{Connection, Item, Message};
use clash::{AuthId, LobbyId};
use log::{debug, error, info, warn};
use rand::{thread_rng, Rng};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::{select, spawn};

pub mod lobby;

use crate::lobby::Lobby;

pub type PlayerMap = HashMap<AuthId, Option<LobbyId>>;
pub type LobbyMap = HashMap<LobbyId, Lobby>;
pub struct State {
    players: PlayerMap,
    lobbies: LobbyMap,
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
        self.players.insert(auth_id, None);
        auth_id
    }

    // TODO: dedupe this.
    fn gen_auth_id(&self) -> AuthId {
        let mut auth_id;
        loop {
            auth_id = thread_rng().gen();
            if !self.players.contains_key(&auth_id) {
                break;
            };
        }
        auth_id
    }

    fn gen_lobby_id(&self) -> LobbyId {
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
        Message::GameHost { auth_id: _ } => {
            let state = &mut *state.write().unwrap();
            if state.players.contains_key(&auth_id) {
                let gen_lobby_id = state.gen_lobby_id();
                state.lobbies.insert(
                    gen_lobby_id,
                    Lobby::new(LobbyOptions::default(), gen_lobby_id, auth_id),
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

            let lobby_id = match state.players.get(&auth_id) {
                Some(Some(id)) => id,
                Some(None) => {
                    error!(
                        "Player id {auth_id:#X} attempted to leave a lobby while not in a lobby"
                    );
                    return true;
                }
                None => {
                    error!("Invalid player id {auth_id:#X}");
                    //TODO: Kick player?
                    return false;
                }
            };

            let lobby = match state.lobbies.get_mut(lobby_id) {
                None => {
                    error!("Attempted to leave lobby with an invalid id '{lobby_id}'");
                    return true;
                }
                Some(l) => l,
            };

            if lobby.is_player_in_lobby(&auth_id) {
                lobby.rem_player(&mut state.players, &auth_id);
            }
        }
        Message::PlayerOptions {
            auth_id: _,
            options,
        } => {
            let state = &mut *state.write().unwrap();

            let lobby_id = match state.players.get(&auth_id) {
                Some(Some(id)) => id,
                Some(None) => {
                    error!("Player id {auth_id:#X} not currently in a lobby");
                    return true;
                }
                None => {
                    error!("Invalid player id {auth_id:#X}");
                    //TODO: Kick player?
                    return false;
                }
            };

            let lobby = match state.lobbies.get_mut(lobby_id) {
                Some(l) => l,
                None => {
                    error!("Invalid lobby id received from player map {:#X}", lobby_id);
                    return true;
                }
            };

            if let Some(player) = lobby.shared.players.get_mut(&auth_id) {
                player.options = options;
                // Temporarily force player to color determined by index
                player.options.color = clash::player::COLORS[0];
            } else {
                error!("Lobby received from player map did not contain player {auth_id:#X}");
                return false;
            }

            if let Some(lobby_send) = lobby_send.as_mut() {
                let message = Message::GameLobbyInfo {
                    auth_id: 0,
                    lobby: lobby.shared.clone(),
                };
                lobby_send.send(message).unwrap();
            }
        }
        Message::GameOptions { auth_id, options } => {
            let state = &mut *state.write().unwrap();
            let lobby_id = match state.players.get(&auth_id) {
                Some(Some(id)) => id,
                Some(None) => {
                    error!("Player id {auth_id:#X} not currently in a lobby");
                    return true;
                }
                None => {
                    error!("Invalid player id {auth_id:#X}");
                    //TODO: Ditto
                    return false;
                }
            };

            let lobby = match state.lobbies.get_mut(lobby_id) {
                Some(l) => {
                    if l.shared.host_id == auth_id {
                        l.shared.options = options;
                    }
                    l
                }
                None => {
                    error!("Invalid lobby id received from player map {:#X}", lobby_id);
                    return true;
                }
            };

            if let Some(lobby_send) = lobby_send {
                let message = Message::GameLobbyInfo {
                    auth_id: 0,
                    lobby: lobby.shared.clone(),
                };
                lobby_send.send(message).unwrap();
            }
        }
        Message::GameCurrentRoom { auth_id: _, room } => {
            let state = &mut *state.write().unwrap();

            let lobby_id = match state.players.get_mut(&auth_id) {
                Some(Some(id)) => id,
                Some(None) => {
                    error!("Player id {auth_id:#X} not currently in a lobby");
                    return true;
                }
                None => {
                    error!("Invalid player id {auth_id:#X}");
                    return false;
                }
            };

            let lobby = match state.lobbies.get_mut(lobby_id) {
                Some(l) => {
                    l.shared
                        .players
                        .get_mut(&auth_id)
                        .expect(
                            "Lobby received from player map did not contain player {auth_id:#X}",
                        )
                        .current_room = room;
                    info!("Player {auth_id:#X} entered {room:?}");
                    l
                }
                None => {
                    error!("Invalid lobby id {lobby_id:#X}");
                    return false;
                }
            };

            if let Some(lobby_send) = lobby_send {
                lobby_send
                    .send(Message::GameLobbyInfo {
                        auth_id: 0,
                        lobby: lobby.shared.clone(),
                    })
                    .unwrap();
            }
        }
        Message::GameItemCollected { auth_id: _, item } => {
            let state = &mut *state.write().unwrap();

            let lobby_id = match state.players.get_mut(&auth_id) {
                Some(Some(id)) => id,
                Some(None) => {
                    error!("Player id {auth_id:#X} not currently in a lobby");
                    return true;
                }
                None => {
                    error!("Invalid player id {auth_id:#X}");
                    return false;
                }
            };

            let lobby = match state.lobbies.get_mut(lobby_id) {
                Some(l) => {
                    if let Item::Spatula(spat) = item {
                        if let Entry::Vacant(e) = l.shared.game_state.spatulas.entry(spat) {
                            e.insert(Some(auth_id));
                            l.shared.players
                                .get_mut(&auth_id)
                                .expect("Lobby received from player map did not contain player {auth_id:#X}")
                                .score += 1;
                            info!("Player {auth_id:#X} collected {spat:?}");
                        }
                    }
                    l
                }
                None => {
                    error!("Invalid lobby id {:#X}", lobby_id);
                    return false;
                }
            };

            if let Some(lobby_send) = lobby_send {
                lobby_send
                    .send(Message::GameLobbyInfo {
                        auth_id: 0,
                        lobby: lobby.shared.clone(),
                    })
                    .unwrap();
            }
        }
        m => {
            warn!("Player id {auth_id:#X} sent a server only message. \nMessage: {m:?}");
        }
    }
    true
}
