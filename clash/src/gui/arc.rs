use std::ops::Range;

use eframe::epaint::{pos2, Color32, CubicBezierShape, Pos2, Shape, Stroke, Vec2};

pub struct ArcShape {
    pub center: Pos2,
    pub radius: f32,
    pub angle: Range<f32>,
    pub stroke: Stroke,
}

impl ArcShape {
    pub fn new(center: Pos2, radius: f32, angle: Range<f32>, stroke: impl Into<Stroke>) -> Self {
        Self {
            center,
            radius,
            angle,
            stroke: stroke.into(),
        }
    }
}

impl From<ArcShape> for Shape {
    fn from(arc: ArcShape) -> Self {
        if arc.angle.is_empty() {
            return Shape::Noop;
        }

        let ArcShape {
            center,
            radius,
            angle,
            stroke,
        } = arc;

        let center = center.to_vec2();
        Shape::CubicBezier(CubicBezierShape::from_points_stroke(
            approx_arc(center, radius, angle),
            false,
            Color32::TRANSPARENT,
            stroke,
        ))
    }
}

/// Given the center and radius of a circle and a range representing the starting and ending angle of an arc of that circle,
/// calculate the points of a cubic bezier curve approximating the arc.
fn approx_arc(center: Vec2, radius: f32, angle: Range<f32>) -> [Pos2; 4] {
    // Calculate the start end points relative to points on a unit-circle
    let srt_x = f32::cos(angle.start);
    let srt_y = f32::sin(angle.start);
    let end_x = f32::cos(angle.end);
    let end_y = f32::sin(angle.end);

    // Calculate the position of those points on the screen.
    let start = pos2(radius * srt_x, radius * srt_y) + center;
    let end = pos2(radius * end_x, radius * end_y) + center;

    // Calculate the control points of the bezier curve
    // The math behind this is described here: https://pomax.github.io/bezierinfo/#circles_cubic,
    // slightly adjusted to account for arcs that don't begin at angle 0
    let angle = angle.end - angle.start;
    let k = (4. / 3.) * f32::tan(angle / 4.);
    let ctrl_1 = pos2(radius * (srt_x - k * srt_y), radius * (srt_y + k * srt_x)) + center;
    let ctrl_2 = pos2(radius * (end_x + k * end_y), radius * (end_y - k * end_x)) + center;

    [start, ctrl_1, ctrl_2, end]
}
