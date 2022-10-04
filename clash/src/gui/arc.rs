use eframe::epaint::{pos2, Color32, CubicBezierShape, Pos2, Shape, Stroke};

pub struct ArcShape {
    pub center: Pos2,
    pub radius: f32,
    pub start_angle: f32,
    pub end_angle: f32,
    pub stroke: Stroke,
}

impl ArcShape {
    pub fn new(
        center: Pos2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        stroke: impl Into<Stroke>,
    ) -> Self {
        Self {
            center,
            radius,
            start_angle,
            end_angle,
            stroke: stroke.into(),
        }
    }
}

impl From<ArcShape> for Shape {
    fn from(arc: ArcShape) -> Self {
        if arc.start_angle == arc.end_angle {
            return Shape::Noop;
        }

        let ArcShape {
            center,
            radius,
            start_angle,
            end_angle,
            stroke,
        } = arc;
        let angle = end_angle - start_angle;
        let k = (4. / 3.) * f32::tan(angle / 4.);
        let srt_x = f32::cos(start_angle);
        let srt_y = f32::sin(start_angle);
        let end_x = f32::cos(end_angle);
        let end_y = f32::sin(end_angle);

        let start = pos2(radius * srt_x, radius * srt_y);
        let ctrl_1 = pos2(radius * (srt_x - k * srt_y), radius * (srt_y + k * srt_x));
        let ctrl_2 = pos2(radius * (end_x + k * end_y), radius * (end_y - k * end_x));
        let end = pos2(radius * end_x, radius * end_y);

        let center = center.to_vec2();
        Shape::CubicBezier(CubicBezierShape::from_points_stroke(
            [
                start + center,
                ctrl_1 + center,
                ctrl_2 + center,
                end + center,
            ],
            false,
            Color32::TRANSPARENT,
            stroke,
        ))
    }
}
