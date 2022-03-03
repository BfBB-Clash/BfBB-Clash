mod game_interface;
mod game_state;

pub use self::game_state::GameStateExt;
pub use game_interface::{GameInterface, InterfaceError, InterfaceResult};

use crate::dolphin::DolphinInterface;
use clash::{lobby::SharedLobby, protocol::Message};
use log::error;
use spin_sleep::LoopHelper;
use std::sync::mpsc::{Receiver, Sender};

pub fn start_game(
    mut gui_sender: Sender<SharedLobby>,
    mut network_sender: tokio::sync::mpsc::Sender<Message>,
    mut logic_receiver: Receiver<Message>,
) {
    let mut loop_helper = LoopHelper::builder()
        .report_interval_s(0.5)
        .build_with_target_rate(126);

    // TODO: Report hooking errors to user/stdout
    let mut game = DolphinInterface::default();
    let _ = game.hook();
    let mut lobby = None;

    loop {
        loop_helper.loop_start();

        // Receive network updates
        update_from_network(&game, &mut lobby, &mut logic_receiver, &mut gui_sender).unwrap();

        if let Some(lobby) = lobby.as_mut() {
            if let Err(InterfaceError::Unhooked) = lobby.update(&game, &mut network_sender) {
                // We lost dolphin
                if lobby.game_state.current_room != None {
                    lobby.game_state.current_room = None;
                    network_sender
                        .blocking_send(Message::GameCurrentRoom {
                            auth_id: 0,
                            room: None,
                        })
                        .unwrap();
                }

                // TODO: Maybe don't re-attempt this every frame
                let _ = game.hook();
            }
        }
        loop_helper.loop_start_s();
    }
}

fn update_from_network<T: GameInterface>(
    _game: &T,
    lobby: &mut Option<SharedLobby>,
    logic_receiver: &mut Receiver<Message>,
    gui_sender: &mut Sender<SharedLobby>,
) -> Result<(), InterfaceError> {
    while let Ok(m) = logic_receiver.try_recv() {
        match m {
            Message::GameBegin { auth_id: _ } => todo!(),
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
                *lobby = Some(new_lobby.clone());
                gui_sender
                    .send(new_lobby)
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
