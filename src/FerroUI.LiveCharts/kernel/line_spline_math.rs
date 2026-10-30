//! Shared cubic spline construction for line, range-line and stacked area series.

use super::Coordinate;
use ferroui_base::Point;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BezierData {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl BezierData {
    pub fn control1(self) -> Point {
        Point::new(self.x0, self.y0)
    }
    pub fn control2(self) -> Point {
        Point::new(self.x1, self.y1)
    }
    pub fn end(self) -> Point {
        Point::new(self.x2, self.y2)
    }
}

pub fn seed_first_segment(current: Coordinate, y_offset: f64) -> BezierData {
    let (x, y) = (current.secondary_value, current.primary_value + y_offset);
    BezierData {
        x0: x,
        y0: y,
        x1: x,
        y1: y,
        x2: x,
        y2: y,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn compute_segment(
    previous: Coordinate,
    pys: f64,
    current: Coordinate,
    cys: f64,
    next: Coordinate,
    nys: f64,
    after_next: Coordinate,
    nnys: f64,
    smoothness: f64,
) -> BezierData {
    let xc1 = (previous.secondary_value + current.secondary_value) / 2.0;
    let yc1 = (previous.primary_value + pys + current.primary_value + cys) / 2.0;
    let xc2 = (current.secondary_value + next.secondary_value) / 2.0;
    let yc2 = (current.primary_value + cys + next.primary_value + nys) / 2.0;
    let xc3 = (next.secondary_value + after_next.secondary_value) / 2.0;
    let yc3 = (next.primary_value + nys + after_next.primary_value + nnys) / 2.0;
    // Preserve the source's float length ratios and its stack-offset expression.
    let length1 = ((current.secondary_value - previous.secondary_value).powi(2)
        + (current.primary_value + cys - previous.primary_value + pys).powi(2))
    .sqrt() as f32;
    let length2 = ((next.secondary_value - current.secondary_value).powi(2)
        + (next.primary_value + nys - current.primary_value + cys).powi(2))
    .sqrt() as f32;
    let length3 = ((after_next.secondary_value - next.secondary_value).powi(2)
        + (after_next.primary_value + nnys - next.primary_value + nys).powi(2))
    .sqrt() as f32;
    let ratio1 = length1 / (length1 + length2);
    let ratio2 = length2 / (length2 + length3);
    let k1 = if ratio1.is_nan() {
        0.0
    } else {
        f64::from(ratio1)
    };
    let k2 = if ratio2.is_nan() {
        0.0
    } else {
        f64::from(ratio2)
    };
    let xm1 = xc1 + (xc2 - xc1) * k1;
    let ym1 = yc1 + (yc2 - yc1) * k1;
    let xm2 = xc2 + (xc3 - xc2) * k2;
    let ym2 = yc2 + (yc3 - yc2) * k2;
    BezierData {
        x0: xm1 + (xc2 - xm1) * smoothness + current.secondary_value - xm1,
        y0: ym1 + (yc2 - ym1) * smoothness + current.primary_value + cys - ym1,
        x1: xm2 + (xc2 - xm2) * smoothness + next.secondary_value - xm2,
        y1: ym2 + (yc2 - ym2) * smoothness + next.primary_value + nys - ym2,
        x2: next.secondary_value,
        y2: next.primary_value + nys,
    }
}

pub fn control_points(
    previous: Point,
    current: Point,
    next: Point,
    after_next: Point,
    smoothness: f64,
) -> BezierData {
    compute_segment(
        Coordinate::new(previous.x, previous.y),
        0.0,
        Coordinate::new(current.x, current.y),
        0.0,
        Coordinate::new(next.x, next.y),
        0.0,
        Coordinate::new(after_next.x, after_next.y),
        0.0,
        smoothness,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_points_produce_finite_degenerate_curve() {
        let c = Coordinate::new(4.0, 8.0);
        let data = compute_segment(c, 0.0, c, 0.0, c, 0.0, c, 0.0, 1.0);
        assert_eq!(data, seed_first_segment(c, 0.0));
    }
    #[test]
    fn zero_smoothness_collapses_controls_to_segment_ends() {
        let d = control_points(
            Point::new(0.0, 3.0),
            Point::new(1.0, 1.0),
            Point::new(2.0, 4.0),
            Point::new(3.0, 2.0),
            0.0,
        );
        assert_eq!(d.control1(), Point::new(1.0, 1.0));
        assert_eq!(d.control2(), Point::new(2.0, 4.0));
    }
    #[test]
    fn symmetric_line_preserves_straight_tangent() {
        let d = control_points(
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(2.0, 2.0),
            Point::new(3.0, 3.0),
            1.0,
        );
        assert_eq!(d.control1(), Point::new(1.5, 1.5));
        assert_eq!(d.control2(), Point::new(1.5, 1.5));
    }
}
