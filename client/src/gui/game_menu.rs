use clash::spatula::Spatula;
use eframe::egui::{Color32, Response, Sense, Ui, Widget};
use strum::IntoEnumIterator;

use crate::game::GameState;

pub struct GameMenu<'a> {
    game: &'a GameState,
}

impl<'a> GameMenu<'a> {
    pub fn new(game: &'a GameState) -> Self {
        Self { game }
    }
}
impl<'a> Widget for GameMenu<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let available_size = ui.available_size();

        // Determine largest radius without overflowing bounds
        let width_radius = (available_size.x - 8. * 7.) / 8. / 2.;
        let height_radius = (available_size.y - 8. * 12.) / 13. / 2.;
        let radius = f32::min(width_radius, height_radius);

        let (rect, response) =
            ui.allocate_exact_size(ui.available_size(), Sense::focusable_noninteractive());

        for spat in Spatula::iter() {
            let color = if self.game.spatulas.contains_key(&spat) {
                Color32::from_rgb(100, 120, 180)
            } else {
                Color32::from_rgb(50, 50, 50)
            };

            let (y, x) = spat.into();
            ui.painter().circle_filled(
                (
                    x as f32 * (2. * radius + 8.) + rect.left_top().x + radius,
                    y as f32 * (2. * radius + 8.) + rect.left_top().y + radius,
                )
                    .into(),
                radius,
                color,
            )
        }

        response
    }
}
