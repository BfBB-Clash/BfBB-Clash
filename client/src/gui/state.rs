use std::cell::Cell;

use eframe::{
    egui::Context,
    epaint::{ColorImage, TextureHandle},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    Lobby,
}

pub struct State {
    pub screen: Cell<Screen>,
    pub logo: TextureHandle,
}

impl State {
    pub fn new(ctx: &Context) -> Self {
        let logo = ctx.load_texture(
            "logo",
            load_image_from_memory(include_bytes!("../../res/logo.png")).unwrap(),
            eframe::egui::TextureFilter::Linear,
        );
        Self {
            screen: Cell::new(Screen::MainMenu),
            logo,
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
