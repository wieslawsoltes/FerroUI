//! The SVG geometry of a series (`GeometrySvg`, which the source parses with
//! `SKPath.ParseSvgPathData` into a `VariableSVGPathGeometry`): the path data read by the path
//! markup parser of FerroUI into the path commands of the scene, quadratic curves and elliptical
//! arcs as cubic curves, and fitted into the rectangle of a visual as `Svg.Draw` does.

use crate::scene::{ChartError, PathCommand};
use ferroui_base::media::{FillRule, PathMarkupParser, SweepDirection};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::{Point, Rect, Size};
use std::f64::consts::{FRAC_PI_2, TAU};

/// Records what the path markup parser draws as path commands.
#[derive(Default)]
struct Recorder {
    commands: Vec<PathCommand>,
    current: Point,
}

impl IGeometryContext for Recorder {
    fn arc_to(
        &mut self,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
        _is_stroked: bool,
    ) {
        let start = self.current;
        arc(
            &mut self.commands,
            start,
            point,
            size,
            rotation_angle,
            is_large_arc,
            sweep_direction == SweepDirection::Clockwise,
        );
        self.current = point;
    }
    fn begin_figure(&mut self, start_point: Point, _is_filled: bool) {
        self.commands.push(PathCommand::MoveTo(start_point));
        self.current = start_point;
    }
    fn cubic_bezier_to(&mut self, control1: Point, control2: Point, end: Point, _: bool) {
        self.commands.push(PathCommand::CubicTo {
            control1,
            control2,
            end,
        });
        self.current = end;
    }
    fn quadratic_bezier_to(&mut self, control: Point, end: Point, _: bool) {
        let start = self.current;
        let toward = |from: Point| {
            Point::new(
                from.x + (control.x - from.x) * 2.0 / 3.0,
                from.y + (control.y - from.y) * 2.0 / 3.0,
            )
        };
        self.commands.push(PathCommand::CubicTo {
            control1: toward(start),
            control2: toward(end),
            end,
        });
        self.current = end;
    }
    fn line_to(&mut self, point: Point, _: bool) {
        self.commands.push(PathCommand::LineTo(point));
        self.current = point;
    }
    fn end_figure(&mut self, is_closed: bool) {
        if is_closed {
            self.commands.push(PathCommand::Close);
        }
    }
    fn set_fill_rule(&mut self, _: FillRule) {}
    fn dispose(&mut self) {}
}

/// The path commands of SVG path data; data the parser rejects is an error, where the source
/// throws.
pub(crate) fn parse(data: &str) -> Result<Vec<PathCommand>, ChartError> {
    let mut recorder = Recorder::default();
    PathMarkupParser::new(&mut recorder)
        .parse(data)
        .map_err(|error| ChartError::InvalidData(format!("invalid GeometrySvg: {error}")))?;
    Ok(recorder.commands)
}

/// The elliptical arc of SVG from `start` to `end` (the endpoint parametrization of the SVG
/// specification, appendix F.6) as cubic curves of at most a quarter turn each.
fn arc(
    commands: &mut Vec<PathCommand>,
    start: Point,
    end: Point,
    radii: Size,
    rotation_degrees: f64,
    large: bool,
    sweep: bool,
) {
    let (mut rx, mut ry) = (radii.width.abs(), radii.height.abs());
    if rx == 0.0 || ry == 0.0 || (start.x == end.x && start.y == end.y) {
        commands.push(PathCommand::LineTo(end));
        return;
    }
    let (sin, cos) = rotation_degrees.to_radians().sin_cos();
    let (dx, dy) = ((start.x - end.x) * 0.5, (start.y - end.y) * 0.5);
    let (x1, y1) = (cos * dx + sin * dy, -sin * dx + cos * dy);
    let lambda = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let numerator = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let denominator = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let root = (numerator / denominator).max(0.0).sqrt() * if large == sweep { -1.0 } else { 1.0 };
    let (cx1, cy1) = (root * rx * y1 / ry, -root * ry * x1 / rx);
    let center = Point::new(
        cos * cx1 - sin * cy1 + (start.x + end.x) * 0.5,
        sin * cx1 + cos * cy1 + (start.y + end.y) * 0.5,
    );
    let angle = |ux: f64, uy: f64| uy.atan2(ux);
    let theta = angle((x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut delta = angle((-x1 - cx1) / rx, (-y1 - cy1) / ry) - theta;
    if sweep && delta < 0.0 {
        delta += TAU;
    } else if !sweep && delta > 0.0 {
        delta -= TAU;
    }
    let at = |t: f64| {
        let (s, c) = t.sin_cos();
        Point::new(
            center.x + rx * cos * c - ry * sin * s,
            center.y + rx * sin * c + ry * cos * s,
        )
    };
    let tangent = |t: f64| {
        let (s, c) = t.sin_cos();
        (-rx * cos * s - ry * sin * c, -rx * sin * s + ry * cos * c)
    };
    let segments = (delta.abs() / FRAC_PI_2).ceil().max(1.0) as usize;
    let step = delta / segments as f64;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    for i in 0..segments {
        let (a, b) = (theta + step * i as f64, theta + step * (i + 1) as f64);
        let (p, q) = (at(a), if i + 1 == segments { end } else { at(b) });
        let (ta, tb) = (tangent(a), tangent(b));
        commands.push(PathCommand::CubicTo {
            control1: Point::new(p.x + k * ta.0, p.y + k * ta.1),
            control2: Point::new(q.x - k * tb.0, q.y - k * tb.1),
            end: q,
        });
    }
}

/// The bounds of the drawn path (`GetTightBounds`): its points and the curves sampled along
/// their length.
pub(crate) fn bounds(commands: &[PathCommand]) -> Rect {
    let mut min = Point::new(f64::INFINITY, f64::INFINITY);
    let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut add = |p: Point| {
        min = Point::new(min.x.min(p.x), min.y.min(p.y));
        max = Point::new(max.x.max(p.x), max.y.max(p.y));
    };
    let mut current = Point::new(0.0, 0.0);
    for command in commands {
        match *command {
            PathCommand::MoveTo(p) | PathCommand::LineTo(p) => {
                add(p);
                current = p;
            }
            PathCommand::CubicTo {
                control1,
                control2,
                end,
            } => {
                for i in 1..=16 {
                    let t = i as f64 / 16.0;
                    let u = 1.0 - t;
                    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                    add(Point::new(
                        a * current.x + b * control1.x + c * control2.x + d * end.x,
                        a * current.y + b * control1.y + c * control2.y + d * end.y,
                    ));
                }
                current = end;
            }
            PathCommand::Close => {}
        }
    }
    if min.x > max.x {
        Rect::new(0.0, 0.0, 0.0, 0.0)
    } else {
        Rect::new(min.x, min.y, max.x - min.x, max.y - min.y)
    }
}

/// The path drawn in `rect` (`Svg.Draw` without `FitToSize`): centred on the rectangle and
/// scaled by its width and height over the larger side of the bounds of the path, so that a path
/// keeps its proportions in a square visual.
pub(crate) fn fit(commands: &[PathCommand], bounds: Rect, rect: Rect) -> Vec<PathCommand> {
    let side = bounds.width.max(bounds.height);
    let (sx, sy) = if side > 0.0 {
        (rect.width / side, rect.height / side)
    } else {
        (0.0, 0.0)
    };
    let (mid_x, mid_y) = (
        bounds.x + bounds.width * 0.5,
        bounds.y + bounds.height * 0.5,
    );
    let (cx, cy) = (rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
    let map = |p: Point| Point::new(cx + (p.x - mid_x) * sx, cy + (p.y - mid_y) * sy);
    commands
        .iter()
        .map(|command| match *command {
            PathCommand::MoveTo(p) => PathCommand::MoveTo(map(p)),
            PathCommand::LineTo(p) => PathCommand::LineTo(map(p)),
            PathCommand::CubicTo {
                control1,
                control2,
                end,
            } => PathCommand::CubicTo {
                control1: map(control1),
                control2: map(control2),
                end: map(end),
            },
            PathCommand::Close => PathCommand::Close,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lines, a closed figure, a relative command and the quadratic curve as a cubic one.
    #[test]
    fn svg_path_data_becomes_path_commands() {
        let commands = parse("M0,0 L10,0 l0,10 Q5,15 0,10 Z").unwrap();
        assert_eq!(commands[0], PathCommand::MoveTo(Point::new(0.0, 0.0)));
        assert_eq!(commands[1], PathCommand::LineTo(Point::new(10.0, 0.0)));
        assert_eq!(commands[2], PathCommand::LineTo(Point::new(10.0, 10.0)));
        let PathCommand::CubicTo { end, .. } = commands[3] else {
            panic!("the quadratic curve")
        };
        assert_eq!(end, Point::new(0.0, 10.0));
        assert_eq!(commands[4], PathCommand::Close);
        assert!(parse("M0,0 X").is_err());
    }

    /// A half circle of radius 10 is two quarter curves through the bottom of the circle, and
    /// its bounds are those of the half disc.
    #[test]
    fn an_arc_is_quarter_curves_on_its_ellipse() {
        let commands = parse("M0,10 A10,10 0 0 0 20,10").unwrap();
        assert_eq!(commands.len(), 3);
        let PathCommand::CubicTo { end, .. } = commands[1] else {
            panic!("a curve")
        };
        assert!(
            (end.x - 10.0).abs() < 1e-9 && (end.y - 20.0).abs() < 1e-9,
            "{end:?}"
        );
        let b = bounds(&commands);
        assert!((b.x - 0.0).abs() < 1e-6 && (b.width - 20.0).abs() < 1e-6);
        assert!((b.y - 10.0).abs() < 1e-6 && (b.height - 10.0).abs() < 0.05);
    }

    /// `Svg.Draw`: the path centred on the rectangle and scaled over the larger side of its
    /// bounds.
    #[test]
    fn a_path_fits_the_middle_of_its_rectangle() {
        let commands = parse("M0,0 L20,0 L20,10 Z").unwrap();
        let fitted = fit(
            &commands,
            bounds(&commands),
            Rect::new(100.0, 100.0, 40.0, 40.0),
        );
        assert_eq!(fitted[0], PathCommand::MoveTo(Point::new(100.0, 110.0)));
        assert_eq!(fitted[2], PathCommand::LineTo(Point::new(140.0, 130.0)));
    }
}
