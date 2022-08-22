use std::collections::HashMap;

use bfbb::{IntoEnumIterator, Spatula};
use clash::{game_state::GameState, player::NetworkedPlayer, PlayerId};
use eframe::{
    egui::{Color32, Response, Sense, Ui, Widget},
    epaint::Vec2,
};

pub struct GameMenu<'a> {
    game: &'a GameState,
    players: &'a HashMap<PlayerId, NetworkedPlayer>,
}

impl<'a> GameMenu<'a> {
    pub fn new(game: &'a GameState, players: &'a HashMap<PlayerId, NetworkedPlayer>) -> Self {
        Self { game, players }
    }
}
impl<'a> Widget for GameMenu<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let available_size = ui.available_size();

        // Determine largest radius without overflowing bounds
        let width_radius = (available_size.x - 8. * 7.) / 8. / 2.;
        let height_radius = (available_size.y - 8. * 12.) / 13. / 2.;
        let radius = f32::min(width_radius, height_radius);
        let desired_size = Vec2::new(radius * 2. * 8. + (8. * 7.), radius * 2. * 13. + (8. * 12.));

        let (rect, response) =
            ui.allocate_exact_size(desired_size, Sense::focusable_noninteractive());

        for spat in Spatula::iter() {
            let color = if let Some(Some(i)) = self.game.spatulas.get(&spat) {
                self.players
                    .get(i)
                    .map(|p| p.options.color())
                    .unwrap_or_default()
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
