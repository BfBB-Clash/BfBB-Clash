mod game_interface;
mod game_state;

use std::sync::mpsc::{Receiver, Sender};

use clash::{protocol::Message, spatula::Spatula};
pub use game_interface::GameInterface;
pub use game_state::GameState;
use spin_sleep::LoopHelper;

use crate::dolphin::DolphinInterface;

pub fn start_game(
    mut gui_sender: Sender<Spatula>,
    mut network_sender: tokio::sync::mpsc::Sender<Message>,
    mut logic_receiver: Receiver<Message>,
) {
    let mut loop_helper = LoopHelper::builder()
        .report_interval_s(0.5)
        .build_with_target_rate(126);
    let mut dolphin = DolphinInterface::default();
    let _ = dolphin.hook();
    let mut game = GameState::default();

    loop {
        loop_helper.loop_start();
        game.update(
            &dolphin,
            &mut gui_sender,
            &mut network_sender,
            &mut logic_receiver,
        );
        loop_helper.loop_sleep();
    }
}
