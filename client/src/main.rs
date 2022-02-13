pub mod dolphin;
pub mod game_interface;

use clash::protocol::Connection;
use eframe::{egui::CentralPanel, epi::App, run_native, NativeOptions};
use tokio::net::TcpStream;

fn main() {
    env_logger::Builder::new()
        .format_level(true)
        .format_module_path(true)
        .format_target(false)
        .format_indent(Some(4))
        .format_timestamp_secs()
        .filter_level(log::LevelFilter::Warn)
        .parse_env("CLASH_LOG")
        .init();

    // Create a new thread and start a tokio runtime on it for talking to the server
    // TODO: Tokio may not be the best tool for the client. It might be better to
    //       simply use std's blocking networking in a new thread, since we should only ever
    //       have a single connection. Unfortunately for now we need to use it since the shared
    //       library is async.
    let _network_thread = std::thread::spawn(start_network);

    // Start gui on the main thread
    start_gui();
}

#[tokio::main(flavor = "current_thread")]
async fn start_network() {
    let sock = TcpStream::connect("127.0.0.1:42932").await.unwrap();
    let mut conn = Connection::new(sock);

    let _accept = conn.read_frame().await.unwrap().unwrap();

    conn.write_frame(clash::protocol::Message::GameHost {
        auth_id: 1,
        lobby_id: 2,
    })
    .await
    .unwrap();
}

struct Clash {
    buf: String,
}

impl App for Clash {
    fn update(&mut self, ctx: &eframe::egui::CtxRef, frame: &eframe::epi::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.text_edit_singleline(&mut self.buf);
        });
    }

    fn name(&self) -> &str {
        "BfBB Clash"
    }
}

fn start_gui() {
    let mut window_options = NativeOptions::default();
    window_options.initial_window_size = Some((600., 720.).into());
    window_options.resizable = false;

    run_native(Box::new(Clash { buf: String::new() }), window_options);
}
