mod game_interface;
mod game_state;

pub use game_interface::GameInterface;
pub use game_state::GameState;
use spin_sleep::LoopHelper;

use crate::dolphin::Dolphin;

pub fn start_game() {
    let mut dolphin = Dolphin::default();
    dolphin.hook();
    let mut game = GameState::new(dolphin);

    let mut loop_helper = LoopHelper::builder()
        .report_interval_s(0.5)
        .build_with_target_rate(126);

    loop {
        loop_helper.loop_start();
        game.update();
        loop_helper.loop_sleep();
    }
}
