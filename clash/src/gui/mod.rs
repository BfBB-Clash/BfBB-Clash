use clash_lib::{lobby::NetworkedLobby, PlayerId};
use eframe::egui::{Response, Ui, Widget, WidgetText};
use eframe::{run_native, IconData, NativeOptions};

use self::clash::Clash;
use self::option_editor::OptionEditor;

mod arc;
mod clash;
mod lobby;
mod main_menu;
mod option_editor;
mod state;
mod val_text;

pub type GuiReceiver = std::sync::mpsc::Receiver<(PlayerId, NetworkedLobby)>;
pub type GuiSender = std::sync::mpsc::Sender<(PlayerId, NetworkedLobby)>;

const BORDER: f32 = 32.;
const PADDING: f32 = 8.;

/// Entry point for the gui. Intended to run on the main thread.
/// Doesn't return until the window is closed.
pub fn run() {
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
        Box::new(|cc| Box::new(Clash::new(cc))),
    );
}

pub trait UiExt<'a, In: ?Sized, Out> {
    fn add_option(
        &mut self,
        label: impl Into<WidgetText>,
        input: &'a mut In,
        on_changed: impl FnMut(&Out) + 'a,
    ) -> Response;
}

impl<'a, In, Out> UiExt<'a, In, Out> for Ui
where
    In: 'a + ?Sized,
    Out: 'a,
    OptionEditor<'a, In, Out>: Widget,
{
    fn add_option(
        &mut self,
        text: impl Into<WidgetText>,
        input: &'a mut In,
        on_changed: impl FnMut(&Out) + 'a,
    ) -> Response {
        let editor = OptionEditor::new(text, input, on_changed);
        self.add(editor)
    }
}
