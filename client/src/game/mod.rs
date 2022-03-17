mod game_interface;
mod game_state;

pub use self::game_state::GameStateExt;
pub use game_interface::{GameInterface, InterfaceError, InterfaceResult};

use crate::dolphin::DolphinInterface;
use clash::{lobby::SharedLobby, protocol::Message, AuthId};
use log::error;
use spin_sleep::LoopHelper;
use std::sync::mpsc::{Receiver, Sender};

pub fn start_game(
    mut gui_sender: Sender<(AuthId, SharedLobby)>,
    mut network_sender: tokio::sync::mpsc::Sender<Message>,
    mut logic_receiver: Receiver<Message>,
) {
    let mut loop_helper = LoopHelper::builder()
        .report_interval_s(0.5)
        .build_with_target_rate(126);

    // TODO: Report hooking errors to user/stdout
    let mut game = DolphinInterface::default();
    let _ = game.hook();
    let mut auth_id = 0;
    let mut lobby = None;

    loop {
        loop_helper.loop_start();

        // Receive network updates
        update_from_network(
            &game,
            &mut auth_id,
            &mut lobby,
            &mut logic_receiver,
            &mut gui_sender,
        )
        .unwrap();

        if let Some(lobby) = lobby.as_mut() {
            if let Err(InterfaceError::Unhooked) = lobby.update(auth_id, &game, &mut network_sender)
            {
                // We lost dolphin
                || -> Option<()> {
                    let local_player = lobby.players.get_mut(&auth_id)?;
                    if local_player.current_room != None {
                        local_player.current_room = None;
                        network_sender
                            .blocking_send(Message::GameCurrentRoom {
                                auth_id: 0,
                                room: None,
                            })
                            .unwrap();
                    }
                    Some(())
                }();

                // TODO: Maybe don't re-attempt this every frame
                let _ = game.hook();
            }
        }
        loop_helper.loop_start_s();
    }
}

fn update_from_network<T: GameInterface>(
    game: &T,
    auth_id: &mut AuthId,
    lobby: &mut Option<SharedLobby>,
    logic_receiver: &mut Receiver<Message>,
    gui_sender: &mut Sender<(AuthId, SharedLobby)>,
) -> Result<(), InterfaceError> {
    while let Ok(m) = logic_receiver.try_recv() {
        match m {
            Message::ConnectionAccept { auth_id: id } => {
                *auth_id = id;
            }
            Message::GameBegin { auth_id: _ } => {
                let _ = game.start_new_game();
                let lobby = lobby
                    .as_mut()
                    .expect("Tried to begin game without being in a lobby");
                lobby.is_started = true;
                gui_sender
                    .send((*auth_id, lobby.clone()))
                    .expect("GUI has crashed and so will we");
            }
            Message::PlayerOptions {
                auth_id: _,
                options: _,
            } => todo!(),
            Message::GameOptions {
                auth_id: _,
                options: _,
            } => todo!(),
            Message::GameLobbyInfo {
                auth_id: _,
                lobby: new_lobby,
            } => {
                // This could fail if the user is restarting dolphin, but that will desync a lot of other things as well
                // so it's fine to just wait for a future lobby update to correct the issue
                let _ = game.set_spatula_count(new_lobby.game_state.spatulas.len() as u32);
                *lobby = Some(new_lobby.clone());
                gui_sender
                    .send((*auth_id, new_lobby))
                    .expect("GUI has crashed and so will we");
            }
            Message::GameForceWarp {
                auth_id: _,
                room: _,
            } => todo!(),
            Message::GameEnd { auth_id: _ } => todo!(),
            Message::GameLeave { auth_id: _ } => todo!(),

            m => {
                error!("Logic received invalid message {m:?}");
            }
        }
    }
    Ok(())
}
