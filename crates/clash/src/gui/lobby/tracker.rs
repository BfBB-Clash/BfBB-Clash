use std::f32::consts::PI;

use bfbb::Spatula;
use clash_lib::{game_state::SpatulaState, lobby::NetworkedLobby, PlayerId};
use eframe::{
    egui::{Color32, Response, Sense, Ui, Widget},
    epaint::{pos2, vec2, Rect, Shape},
};

use crate::gui::{arc::ArcShape, state::State};

const GOLD: Color32 = Color32::from_rgb(0xd4, 0xaf, 0x37);
const SILVER: Color32 = Color32::from_rgb(0xe0, 0xe0, 0xe0);

pub struct Tracker<'a> {
    state: &'a State,
    lobby: &'a NetworkedLobby,
    local_player: PlayerId,
}

impl<'a> Tracker<'a> {
    pub fn new(state: &'a State, lobby: &'a NetworkedLobby, local_player: PlayerId) -> Self {
        Self {
            state,
            lobby,
            local_player,
        }
    }
}
impl<'a> Tracker<'a> {
    pub fn ui(self, ui: &mut Ui) {
        for x in 0..13 {
            ui.horizontal(|ui| {
                for y in 0..8 {
                    if let Ok(spat) = Spatula::try_from((x, y)) {
                        ui.add(SpatulaStatus::new(
                            self.state,
                            spat,
                            self.local_player,
                            self.lobby,
                        ))
                        .on_hover_text(format!("{spat:?}"));
                    }
                }
            });
        }
    }
}

struct SpatulaStatus<'a> {
    state: &'a State,
    spat: Spatula,
    local_player: PlayerId,
    lobby: &'a NetworkedLobby,
}

impl<'a> SpatulaStatus<'a> {
    fn new(
        state: &'a State,
        spat: Spatula,
        local_player: PlayerId,
        lobby: &'a NetworkedLobby,
    ) -> Self {
        Self {
            state,
            spat,
            local_player,
            lobby,
        }
    }
}

impl Widget for SpatulaStatus<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            state: app_state,
            spat,
            local_player,
            lobby,
        } = self;

        let def = SpatulaState::default();
        let state = self.lobby.game_state.spatulas.get(&spat).unwrap_or(&def);
        let radius = 16.15;
        let (rect, response) =
            ui.allocate_exact_size(vec2(radius * 2., radius * 2.), Sense::hover());

        let (texture, color) = if state.collection_vec.contains(&local_player)
            || state.collection_vec.len() == lobby.options.tier_count.into()
        {
            (&app_state.golden_spatula, GOLD)
        } else {
            (&app_state.silver_spatula, SILVER)
        };
        if app_state.use_icons.get() {
            ui.painter().add(Shape::image(
                texture.id(),
                rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            ));
        } else {
            ui.painter().circle_filled(rect.center(), radius, color);
        }

        match lobby.options.tier_count {
            1 => {
                let color = state
                    .collection_vec
                    .first()
                    .and_then(|id| lobby.players.get(id))
                    .map(|p| p.options.color)
                    .map(|(r, g, b)| Color32::from_rgb(r, g, b))
                    .unwrap_or(ui.style().visuals.widgets.inactive.bg_fill);
                ui.painter()
                    .circle_stroke(rect.center(), radius + 2., (1., color));
            }
            x => {
                let ang = PI * 2. / x as f32;
                let pad = f32::to_radians(10.0);
                for i in 0..x {
                    let color = state
                        .collection_vec
                        .get(usize::from(i))
                        .and_then(|id| lobby.players.get(id))
                        .map(|p| p.options.color)
                        .map(|(r, g, b)| Color32::from_rgb(r, g, b))
                        .unwrap_or(ui.style().visuals.widgets.inactive.bg_fill);
                    let start = ang * i as f32 + pad / 2.;
                    ui.painter().add(ArcShape::new(
                        rect.center(),
                        radius + 2.,
                        start..start + ang - pad,
                        (1., color),
                    ));
                }
            }
        }

        response
    }
}
