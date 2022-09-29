use std::cell::RefCell;

use eframe::{
    egui::Context,
    epaint::{ColorImage, TextureHandle},
};

use super::lobby::LobbyData;

pub type ErrorSender = std::sync::mpsc::Sender<anyhow::Error>;
pub type ErrorReceiver = std::sync::mpsc::Receiver<anyhow::Error>;

#[derive(Debug)]
pub enum Screen {
    MainMenu(Submenu),
    Lobby(LobbyData),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Submenu {
    Root,
    Host,
    Join,
}

pub struct State {
    pub screen: RefCell<Screen>,
    pub logo: TextureHandle,
    pub error_sender: ErrorSender,
    pub error_receiver: ErrorReceiver,
}

impl State {
    pub fn new(ctx: &Context) -> Self {
        let logo = ctx.load_texture(
            "logo",
            load_image_from_memory(include_bytes!("../../res/logo.png")).unwrap(),
            eframe::egui::TextureFilter::Linear,
        );
        let (error_sender, error_receiver) = std::sync::mpsc::channel();
        Self {
            screen: RefCell::new(Screen::MainMenu(Submenu::Root)),
            logo,
            error_sender,
            error_receiver,
        }
    }
}

fn load_image_from_memory(image_data: &[u8]) -> Result<ColorImage, image::ImageError> {
    let image = image::load_from_memory(image_data)?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
}
