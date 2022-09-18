use std::error::Error;
use std::sync::mpsc::Receiver;

use ::clash::lobby::NetworkedLobby;
use ::clash::net::Message;
use ::clash::PlayerId;
use eframe::{run_native, IconData, NativeOptions};

use self::clash::Clash;

mod clash;
mod lobby;
mod main_menu;
mod state;
mod val_text;

const BORDER: f32 = 32.;
const PADDING: f32 = 8.;

/// Entry point for the gui. Intended to run on the main thread.
/// Doesn't return until the window is closed.
pub fn run(
    gui_receiver: Receiver<(PlayerId, NetworkedLobby)>,
    error_receiver: Receiver<Box<dyn Error + Send>>,
    network_sender: tokio::sync::mpsc::Sender<Message>,
) {
    let icon_bytes = include_bytes!("../../res/icon.ico");
    let icon = image::load_from_memory(icon_bytes).unwrap().to_rgba8();
    let (width, height) = icon.dimensions();

    let window_options = NativeOptions {
        initial_window_size: Some((600., 742.).into()),
        resizable: false,
        icon_data: Some(IconData {
            rgba: icon.into_raw(),
            width,
            height,
        }),
        ..Default::default()
    };

    run_native(
        "BfBB Clash",
        window_options,
        Box::new(|cc| Box::new(Clash::new(cc, gui_receiver, error_receiver, network_sender))),
    );
}
