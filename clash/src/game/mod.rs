mod game_mode;
mod game_state;

use bfbb::game_interface::{dolphin::DolphinInterface, GameInterface, InterfaceError};
use bfbb::{EnumCount, Spatula};
use clash_lib::{lobby::NetworkedLobby, net::Message, PlayerId};
use log::error;
use spin_sleep::LoopHelper;
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};

use self::{game_mode::GameMode, game_state::ClashGame};

pub fn start_game(
    mut gui_sender: Sender<(PlayerId, NetworkedLobby)>,
    mut network_sender: tokio::sync::mpsc::Sender<Message>,
    mut logic_receiver: Receiver<Message>,
) {
    let mut loop_helper = LoopHelper::builder()
        .report_interval_s(0.5)
        .build_with_target_rate(126);

    // TODO: Report hooking errors to user/stdout
    let mut interface = DolphinInterface::default();
    let _ = interface.hook();
    let mut game = ClashGame;
    let mut player_id = 0;
    let mut lobby = None;
    // Not sure if this is the best approach, the idea is that it would be faster
    // to store a local state of what we as a client have collected rather
    // than searching to see if we've collected it.
    let mut local_spat_state = HashSet::<Spatula>::with_capacity(Spatula::COUNT);

    loop {
        loop_helper.loop_start();
        // You have to call this to avoid overflowing an integer within the loop helper
        let _ = loop_helper.report_rate();

        // Receive network updates
        update_from_network(
            &interface,
            &mut player_id,
            &mut lobby,
            &mut logic_receiver,
            &mut gui_sender,
            &mut local_spat_state,
        )
        .unwrap();

        if let Some(lobby) = lobby.as_mut() {
            if let Err(InterfaceError::Unhooked) = game.update(
                &interface,
                lobby,
                player_id,
                &mut network_sender,
                &mut local_spat_state,
            ) {
                // We lost dolphin
                if let Some(local_player) = lobby.players.get_mut(&player_id) {
                    if local_player.current_level != None {
                        local_player.current_level = None;
                        network_sender
                            .try_send(Message::GameCurrentLevel { level: None })
                            .unwrap();
                        network_sender
                            .blocking_send(Message::PlayerCanStart(false))
                            .unwrap();
                    }
                }

                // TODO: Maybe don't re-attempt this every frame
                let _ = interface.hook();
            }
        }
        loop_helper.loop_start_s();
    }
}

fn update_from_network<T: GameInterface>(
    game: &T,
    player_id: &mut PlayerId,
    lobby: &mut Option<NetworkedLobby>,
    logic_receiver: &mut Receiver<Message>,
    gui_sender: &mut Sender<(PlayerId, NetworkedLobby)>,
    local_spat_state: &mut HashSet<Spatula>,
) -> Result<(), InterfaceError> {
    while let Ok(m) = logic_receiver.try_recv() {
        match m {
            Message::ConnectionAccept { player_id: id } => {
                *player_id = id;
            }
            Message::GameBegin => {
                local_spat_state.clear();
                let _ = game.start_new_game();
                let lobby = lobby
                    .as_mut()
                    .expect("Tried to begin game without being in a lobby");

                if lobby.options.ng_plus {
                    let _ = game.unlock_powers();
                }
                gui_sender
                    .send((*player_id, lobby.clone()))
                    .expect("GUI has crashed and so will we");
            }
            Message::PlayerOptions { options: _ } => todo!(),
            Message::GameOptions { options: _ } => todo!(),
            Message::GameLobbyInfo { lobby: new_lobby } => {
                // This could fail if the user is restarting dolphin, but that will desync a lot of other things as well
                // so it's fine to just wait for a future lobby update to correct the issue
                let _ = game.set_spatula_count(new_lobby.game_state.spatulas.len() as u32);
                *lobby = Some(new_lobby.clone());
                gui_sender
                    .send((*player_id, new_lobby))
                    .expect("GUI has crashed and so will we");
            }
            Message::GameForceWarp { level: _ } => todo!(),
            Message::GameEnd => todo!(),
            Message::GameLeave => todo!(),

            m => {
                error!("Logic received invalid message {m:?}");
            }
        }
    }
    Ok(())
}
