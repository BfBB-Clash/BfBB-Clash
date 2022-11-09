#![cfg_attr(
    all(target_os = "windows", not(debug_assertions), not(feature = "console")),
    windows_subsystem = "windows"
)]

use tracing::metadata::LevelFilter;

mod game;
mod gui;
mod net;

const VERSION: &str = env!("CLASH_VERSION");

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::DEBUG)
        .init();

    // Start gui on the main thread
    gui::run();
}
