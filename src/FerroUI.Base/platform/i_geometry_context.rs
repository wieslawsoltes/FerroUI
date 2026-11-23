use crate::media::{FillRule, SweepDirection};
use crate::{Point, Size};

/// Describes a geometry using drawing commands.
///
/// A context is used by one owner at a time and must be disposed when the
/// description is complete.
pub trait IGeometryContext {
    /// Draws an arc to the specified point.
    ///
    /// `size` holds the radii of the ellipse whose path is used to draw the
    /// arc and `rotation_angle` the rotation of that ellipse in degrees
    /// (positive values are clockwise). `is_large_arc` selects the arc
    /// greater than 180 degrees.
    fn arc_to(
        &mut self,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
        is_stroked: bool,
    );

    /// Begins a new figure.
    fn begin_figure(&mut self, start_point: Point, is_filled: bool);

    /// Draws a Bezier curve to the specified point.
    fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool);

    /// Draws a quadratic Bezier curve to the specified point.
    fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, is_stroked: bool);

    /// Draws a line to the specified point.
    fn line_to(&mut self, point: Point, is_stroked: bool);

    /// Draws a line to each of `points` in turn, as [`line_to`](Self::line_to) for every point
    /// does. A backend that can take a run of points in one step does (a FerroUI addition,
    /// docs/porting/desktop-performance.md, section 9.95): a figure of many points, a polyline
    /// or a polygon, then crosses into the backend once and not once a point.
    fn lines_to(&mut self, points: &[Point], is_stroked: bool) {
        for point in points {
            self.line_to(*point, is_stroked);
        }
    }

    /// Ends the figure started by [`begin_figure`](Self::begin_figure).
    fn end_figure(&mut self, is_closed: bool);

    /// Sets the fill rule.
    fn set_fill_rule(&mut self, fill_rule: FillRule);

    /// Completes the description and releases the context.
    fn dispose(&mut self);
}
