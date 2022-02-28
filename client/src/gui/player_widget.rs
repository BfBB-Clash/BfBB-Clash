use clash::room::Room;
use eframe::egui::{Color32, Response, Sense, Stroke, TextStyle, Ui, Vec2, Widget};

pub struct PlayerUi<'a> {
    name: &'a str,
    score: u32,
    location: Option<Room>,
    color: Color32,
}

impl<'a> PlayerUi<'a> {
    pub fn new(name: &'a str, score: u32, location: Option<Room>, color: Color32) -> Self {
        Self {
            name,
            score,
            location,
            color,
        }
    }
}

impl<'a> Widget for PlayerUi<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let PlayerUi {
            name,
            score,
            location,
            color,
        } = self;

        // Use individual layouts instead of a single one to be able to add padding between each line
        let name_galley = ui.painter().layout_no_wrap(
            name.to_string(),
            TextStyle::Body.resolve(ui.style()),
            color,
        );
        let score_galley = ui.painter().layout_no_wrap(
            format!("Spatulas: {score}"),
            TextStyle::Body.resolve(ui.style()),
            color,
        );
        let room_galley = ui.painter().layout_no_wrap(
            location
                .map(|r| r.to_string())
                .unwrap_or_else(|| "? ? ?".to_string()),
            TextStyle::Small.resolve(ui.style()),
            color,
        );

        let name_size = name_galley.size();
        let score_size = score_galley.size();
        let room_size = room_galley.size();
        // Use longest level name for the overall width
        let longest_width = ui
            .painter()
            .layout_no_wrap(
                Room::MermalairVillianContainment.to_string(),
                TextStyle::Small.resolve(ui.style()),
                color,
            )
            .size()
            .x;

        let padding = ui.spacing().button_padding;
        let desired_size = Vec2::new(
            longest_width + 4. * padding.x,
            name_size.y + score_size.y + room_size.y + 4. * padding.y,
        );
        let (rect, response) =
            ui.allocate_exact_size(desired_size, Sense::focusable_noninteractive());

        if ui.is_rect_visible(rect) {
            ui.painter().rect_stroke(rect, 0., Stroke::new(2., color));
            let mut text_pos = rect.left_top() + padding;

            ui.painter().galley(text_pos, name_galley);
            text_pos.y += name_size.y + padding.y;

            ui.painter().galley(text_pos, score_galley);
            text_pos.y += score_size.y + padding.y;

            ui.painter().galley(text_pos, room_galley);
        }

        response
    }
}
