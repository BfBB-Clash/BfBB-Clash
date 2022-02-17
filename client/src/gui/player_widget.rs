use egui::{Align2, Color32, Response, Sense, Stroke, TextStyle, Vec2, Widget, WidgetText};

pub struct PlayerUi {
    name: WidgetText,
    score: u32,
    location: WidgetText,
    color: Color32,
}

impl PlayerUi {
    pub fn new(name: WidgetText, score: u32, location: WidgetText, color: Color32) -> Self {
        Self {
            name,
            score,
            location,
            color,
        }
    }
}

impl Widget for PlayerUi {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let PlayerUi {
            name,
            score,
            location,
            color,
        } = self;

        // TODO: Use TextGalleys and avoid hardcoding height.
        let desired_size = (ui.available_width(), 110.).into();
        let (rect, response) =
            ui.allocate_exact_size(desired_size, Sense::focusable_noninteractive());

        let offset = Vec2::new(4., 0.);
        let name_pos = rect.left_top() + offset;
        let spat_pos = rect.left_center() + offset;
        let location_pos = rect.left_bottom() + offset;

        if ui.is_rect_visible(rect) {
            ui.painter().rect_stroke(rect, 0., Stroke::new(2., color));
            ui.painter().text(
                name_pos,
                Align2::LEFT_TOP,
                name.text(),
                TextStyle::Body,
                color,
            );
            ui.painter().text(
                spat_pos,
                Align2::LEFT_CENTER,
                format!("Spatulas: {score}"),
                TextStyle::Body,
                color,
            );
            ui.painter().text(
                location_pos,
                Align2::LEFT_BOTTOM,
                location.text(),
                TextStyle::Small,
                color,
            );
        }

        response
    }
}
