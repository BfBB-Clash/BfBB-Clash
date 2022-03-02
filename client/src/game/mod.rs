mod game_interface;
mod game_state;

pub use self::game_state::GameStateExt;
pub use game_interface::{GameInterface, InterfaceError, InterfaceResult};

use crate::{dolphin::DolphinInterface, gui::GuiMessage};
use clash::{
    game_state::GameState,
    lobby::{LobbyOptions, SharedLobby},
    protocol::Message,
};

use spin_sleep::LoopHelper;
use std::sync::mpsc::{Receiver, Sender};

pub fn start_game(
    mut gui_sender: Sender<GuiMessage>,
    mut network_sender: tokio::sync::mpsc::Sender<Message>,
    mut logic_receiver: Receiver<Message>,
) {
    let mut loop_helper = LoopHelper::builder()
        .report_interval_s(0.5)
        .build_with_target_rate(126);

    // TODO: Report hooking errors to user/stdout
    let mut dolphin = DolphinInterface::default();
    let _ = dolphin.hook();
    let mut game = GameState::new(SharedLobby::new(0, LobbyOptions::default(), None));

    loop {
        loop_helper.loop_start();
        if let Err(InterfaceError::Unhooked) = game.update(
            &dolphin,
            &mut gui_sender,
            &mut network_sender,
            &mut logic_receiver,
        ) {
            // Attempt to rehook
            let _ = gui_sender.send(GuiMessage::Room(None));
            let _ = dolphin.hook();
        }
        loop_helper.loop_start_s();
    }
}
