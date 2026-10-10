//! Port of `Rendering/SceneGraph/LineBoundsHelper.cs` of the base library, which the project
//! file of the upstream sample links into the sample (`LineBoundsHelper.cs`): the helper is
//! internal to the base library, here as there, so the sample has its own copy.

use ferroui_base::media::{IPen, PenLineCap};
use ferroui_base::{Point, Rect};

pub(crate) struct LineBoundsHelper;

impl LineBoundsHelper {
    fn calculate_angle(p1: Point, p2: Point) -> f64 {
        let x_diff = p2.x - p1.x;
        let y_diff = p2.y - p1.y;

        y_diff.atan2(x_diff)
    }

    pub(crate) fn calculate_opp_side(angle: f64, hyp: f64) -> f64 {
        angle.sin() * hyp
    }

    pub(crate) fn calculate_adj_side(angle: f64, hyp: f64) -> f64 {
        angle.cos() * hyp
    }

    fn translate_points_along_tangent(p1: Point, p2: Point, angle: f64, distance: f64) -> (Point, Point) {
        let x_diff = Self::calculate_opp_side(angle, distance);
        let y_diff = Self::calculate_adj_side(angle, distance);

        let c1 = Point::new(p1.x + x_diff, p1.y - y_diff);
        let c2 = Point::new(p1.x - x_diff, p1.y + y_diff);

        let c3 = Point::new(p2.x + x_diff, p2.y - y_diff);
        let c4 = Point::new(p2.x - x_diff, p2.y + y_diff);

        let min_x = c1.x.min(c2.x.min(c3.x.min(c4.x)));
        let min_y = c1.y.min(c2.y.min(c3.y.min(c4.y)));
        let max_x = c1.x.max(c2.x.max(c3.x.max(c4.x)));
        let max_y = c1.y.max(c2.y.max(c3.y.max(c4.y)));

        (Point::new(min_x, min_y), Point::new(max_x, max_y))
    }

    fn calculate_bounds_with_angle(p1: Point, p2: Point, thickness: f64, angle_to_corner: f64) -> Rect {
        let pts = Self::translate_points_along_tangent(p1, p2, angle_to_corner, thickness / 2.0);

        Rect::from_points(pts.0, pts.1)
    }

    pub(crate) fn calculate_bounds(p1: Point, p2: Point, p: &dyn IPen) -> Rect {
        let radians = Self::calculate_angle(p1, p2);

        if p.line_cap() != PenLineCap::Flat {
            let pts =
                Self::translate_points_along_tangent(p1, p2, radians - std::f64::consts::PI / 2.0, p.thickness() / 2.0);

            Self::calculate_bounds_with_angle(pts.0, pts.1, p.thickness(), radians)
        } else {
            Self::calculate_bounds_with_angle(p1, p2, p.thickness(), radians)
        }
    }
}
