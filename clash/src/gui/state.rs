use std::cell::Cell;

use eframe::{
    egui::{Context, TextureFilter},
    epaint::{ColorImage, TextureHandle},
    App,
};

pub type ErrorSender = std::sync::mpsc::Sender<anyhow::Error>;
pub type ErrorReceiver = std::sync::mpsc::Receiver<anyhow::Error>;

pub struct State {
    next_app: Cell<Option<Box<dyn App>>>,
    pub logo: TextureHandle,
    pub golden_spatula: TextureHandle,
    pub silver_spatula: TextureHandle,
    pub error_sender: ErrorSender,
    pub error_receiver: ErrorReceiver,
}

impl State {
    pub fn new(ctx: &Context) -> Self {
        let logo = ctx.load_texture(
            "logo",
            load_image_from_memory(include_bytes!("../../res/logo.png"))
                .expect("The image is in the binary."),
            TextureFilter::Linear,
        );
        let golden_spatula = ctx.load_texture(
            "golden spatula",
            load_image_from_memory(include_bytes!("../../res/golden_spatula_60.gif"))
                .expect("The image is in the binary"),
            TextureFilter::Linear,
        );
        let silver_spatula = ctx.load_texture(
            "silver spatula",
            load_image_from_memory(include_bytes!("../../res/silver_spatula_60.gif"))
                .expect("The image is in the binary."),
            TextureFilter::Linear,
        );
        let (error_sender, error_receiver) = std::sync::mpsc::channel();
        Self {
            next_app: Cell::new(None),
            logo,
            golden_spatula,
            silver_spatula,
            error_sender,
            error_receiver,
        }
    }

    pub fn change_app(&self, new_app: impl App + 'static) {
        self.next_app.set(Some(Box::new(new_app)));
    }

    pub fn get_new_app(&self) -> Option<Box<dyn App>> {
        self.next_app.take()
    }
}

fn load_image_from_memory(image_data: &[u8]) -> Result<ColorImage, image::ImageError> {
    let image = image::load_from_memory(image_data)?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
}
