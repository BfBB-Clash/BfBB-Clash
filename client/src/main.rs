#![cfg_attr(
    all(target_os = "windows", not(feature = "console")),
    windows_subsystem = "windows"
)]

use std::{error::Error, sync::mpsc::channel};

use clash::net::Message;

mod game;
mod gui;
mod net;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    env_logger::Builder::new()
        .format_level(true)
        .format_module_path(true)
        .format_target(false)
        .format_indent(Some(4))
        .format_timestamp_secs()
        .filter_level(log::LevelFilter::Debug)
        .parse_env("CLASH_LOG")
        .init();

    let (network_sender, network_receiver) = tokio::sync::mpsc::channel::<Message>(100);
    let (logic_sender, logic_receiver) = channel::<Message>();
    let (error_sender, error_receiver) = channel::<Box<dyn Error + Send>>();
    // Create a new thread and start a tokio runtime on it for talking to the server
    // TODO: Tokio may not be the best tool for the client. It might be better to
    //       simply use std's blocking networking in a new thread, since we should only ever
    //       have a single connection. Unfortunately for now we need to use it since the shared
    //       library is async.
    let _network_thread = std::thread::Builder::new()
        .name("Network".into())
        .spawn(move || net::run(network_receiver, logic_sender, error_sender));

    // Start Game Thread
    let (gui_sender, gui_receiver) = channel();
    let _game_thread = {
        let network_sender = network_sender.clone();
        std::thread::Builder::new()
            .name("Logic".into())
            .spawn(move || game::start_game(gui_sender, network_sender, logic_receiver))
    };

    // Start gui on the main thread
    gui::run(gui_receiver, error_receiver, network_sender);
}
