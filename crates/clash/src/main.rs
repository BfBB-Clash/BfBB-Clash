#![cfg_attr(
    all(target_os = "windows", not(debug_assertions), not(feature = "console")),
    windows_subsystem = "windows"
)]

mod game;
mod gui;
mod net;

const VERSION: &str = env!("CLASH_VERSION");

fn main() {
    // env_logger::Builder::new()
    //     .format_level(true)
    //     .format_module_path(true)
    //     .format_target(false)
    //     .format_indent(Some(4))
    //     .format_timestamp_secs()
    //     .filter_level(log::LevelFilter::Debug)
    //     .parse_env("CLASH_LOG")
    //     .init();
    tracing_subscriber::fmt().init();

    // Start gui on the main thread
    gui::run();
}
