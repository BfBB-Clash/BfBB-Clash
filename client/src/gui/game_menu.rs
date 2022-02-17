use clash::spatula::Spatula;
use egui::{Color32, Sense, Widget};
use strum::IntoEnumIterator;

pub struct GameMenu {}

impl GameMenu {
    pub fn new() -> Self {
        Self {}
    }
}
impl Widget for GameMenu {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let available_size = ui.available_size();

        // Determine largest radius without overflowing bounds
        let width_radius = (available_size.x - 8. * 7.) / 8. / 2.;
        let height_radius = (available_size.y - 8. * 12.) / 13. / 2.;
        let radius = f32::min(width_radius, height_radius);

        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), Sense::focusable_noninteractive());

        for spat in Spatula::iter() {
            let (y, x) = spat.into();
            ui.painter().circle_filled(
                (
                    x as f32 * (2. * radius + 8.) + rect.left_top().x + radius,
                    y as f32 * (2. * radius + 8.) + rect.left_top().y + radius,
                )
                    .into(),
                radius,
                Color32::from_rgb(50, 50, 50),
            )
        }

        response
    }
}
