use bfbb::{IntoEnumIterator, Spatula};
use clash_lib::{game_state::SpatulaState, lobby::NetworkedLobby, PlayerId};
use eframe::{
    egui::{Color32, Response, Sense, Ui, Widget},
    epaint::Vec2,
};

const GOLD: Color32 = Color32::from_rgb(0xd4, 0xaf, 0x37);
const SILVER: Color32 = Color32::from_rgb(0xe0, 0xe0, 0xe0);
const DISABLED: Color32 = Color32::from_rgb(0x3c, 0x3c, 0x3c);

pub struct Tracker<'a> {
    lobby: &'a NetworkedLobby,
    local_player: PlayerId,
}

impl<'a> Tracker<'a> {
    pub fn new(lobby: &'a NetworkedLobby, local_player: PlayerId) -> Self {
        Self {
            lobby,
            local_player,
        }
    }
}
impl<'a> Widget for Tracker<'a> {
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
            let spat_state = self
                .lobby
                .game_state
                .spatulas
                .get(&spat)
                .unwrap_or(&SpatulaState::default())
                .clone();
            // TODO: issue #57, Make spatula gold if locally collected and grayed out if unavailable
            let color = if spat_state.collection_vec.contains(&self.local_player) {
                GOLD
            } else if spat_state.collection_vec.len() == self.lobby.options.tier_count.into() {
                DISABLED
            } else {
                SILVER
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
