mod game_interface;
mod game_state;

use std::time::Duration;

pub use game_interface::GameInterface;
pub use game_state::GameState;

use crate::dolphin::Dolphin;

pub fn start_game() {
    let mut dolphin = Dolphin::default();
    dolphin.hook();
    let mut game = GameState::new(dolphin);

    loop {
        game.update();
        std::thread::sleep(Duration::from_millis(8));
    }
}
