//! Path data and the basic shapes, as the commands of a stream geometry.
//!
//! Arcs become cubic curves here, so that every command can be transformed point by point (for a
//! clip) and measured (for a bounding box), and no backend sees a degenerate arc.

use crate::numbers::Scanner;
use ferroui_base::media::{FillRule, PathCommand};
use ferroui_base::{Matrix, Point, Rect};
use std::f64::consts::{FRAC_PI_2, TAU};

/// Writes the commands of a path.
pub(crate) struct PathBuilder {
    commands: Vec<PathCommand>,
    is_open: bool,
    start: Point,
    current: Point,
}

impl PathBuilder {
    pub(crate) fn new(fill_rule: FillRule) -> Self {
        Self {
            commands: vec![PathCommand::SetFillRule(fill_rule)],
            is_open: false,
            start: Point::new(0.0, 0.0),
            current: Point::new(0.0, 0.0),
        }
    }

    fn end_figure(&mut self, is_closed: bool) {
        if self.is_open {
            self.commands.push(PathCommand::EndFigure(is_closed));
            self.is_open = false;
        }
    }

    /// A figure at the current point, after a close that no move followed.
    fn ensure_figure(&mut self) {
        if !self.is_open {
            self.commands.push(PathCommand::BeginFigure(self.current, true));
            self.start = self.current;
            self.is_open = true;
        }
    }

    pub(crate) fn move_to(&mut self, point: Point) {
        self.end_figure(false);
        self.current = point;
        self.ensure_figure();
    }

    pub(crate) fn line_to(&mut self, point: Point) {
        self.ensure_figure();
        self.commands.push(PathCommand::LineTo(point, true));
        self.current = point;
    }

    fn quadratic_to(&mut self, control: Point, point: Point) {
        self.ensure_figure();
        self.commands.push(PathCommand::QuadraticBezierTo(control, point, true));
        self.current = point;
    }

    fn cubic_to(&mut self, first: Point, second: Point, point: Point) {
        self.ensure_figure();
        self.commands.push(PathCommand::CubicBezierTo(first, second, point, true));
        self.current = point;
    }

    pub(crate) fn close(&mut self) {
        self.end_figure(true);
        self.current = self.start;
    }

    /// An elliptical arc to `to`, as cubic curves (SVG 2, appendix B.2.4).
    pub(crate) fn arc_to(&mut self, radii: (f64, f64), rotation: f64, is_large: bool, is_sweep: bool, to: Point) {
        let from = self.current;
        if from == to {
            return;
        }
        let (mut rx, mut ry) = (radii.0.abs(), radii.1.abs());
        if rx == 0.0 || ry == 0.0 {
            self.line_to(to);
            return;
        }
        let (sin, cos) = rotation.to_radians().sin_cos();
        let (half_x, half_y) = ((from.x - to.x) / 2.0, (from.y - to.y) / 2.0);
        let (x1, y1) = (cos * half_x + sin * half_y, -sin * half_x + cos * half_y);
        let reach = (x1 / rx).powi(2) + (y1 / ry).powi(2);
        if reach > 1.0 {
            let grow = reach.sqrt();
            rx *= grow;
            ry *= grow;
        }
        let (rx2, ry2, x2, y2) = (rx * rx, ry * ry, x1 * x1, y1 * y1);
        let denominator = rx2 * y2 + ry2 * x2;
        let root = ((rx2 * ry2 - denominator) / denominator).max(0.0).sqrt();
        let co = if is_large == is_sweep { -root } else { root };
        let (cx1, cy1) = (co * rx * y1 / ry, -co * ry * x1 / rx);
        let center = Point::new(
            cos * cx1 - sin * cy1 + (from.x + to.x) / 2.0,
            sin * cx1 + cos * cy1 + (from.y + to.y) / 2.0,
        );
        let first = ((y1 - cy1) / ry).atan2((x1 - cx1) / rx);
        let mut delta = ((-y1 - cy1) / ry).atan2((-x1 - cx1) / rx) - first;
        if is_sweep && delta < 0.0 {
            delta += TAU;
        } else if !is_sweep && delta > 0.0 {
            delta -= TAU;
        }
        if !delta.is_finite() {
            self.line_to(to);
            return;
        }
        let at = |angle: f64| {
            let (s, c) = angle.sin_cos();
            Point::new(center.x + rx * c * cos - ry * s * sin, center.y + rx * c * sin + ry * s * cos)
        };
        let tangent = |angle: f64| {
            let (s, c) = angle.sin_cos();
            (-rx * s * cos - ry * c * sin, -rx * s * sin + ry * c * cos)
        };
        // At most four: the sweep is within a full turn.
        let segments = (delta.abs() / FRAC_PI_2).ceil().clamp(1.0, 4.0) as u32;
        let step = delta / f64::from(segments);
        let handle = 4.0 / 3.0 * (step / 4.0).tan();
        let mut start = first;
        let mut point = from;
        for index in 0..segments {
            let end = start + step;
            let target = if index + 1 == segments { to } else { at(end) };
            let (dx0, dy0) = tangent(start);
            let (dx1, dy1) = tangent(end);
            self.cubic_to(
                Point::new(point.x + handle * dx0, point.y + handle * dy0),
                Point::new(target.x - handle * dx1, target.y - handle * dy1),
                target,
            );
            point = target;
            start = end;
        }
    }

    pub(crate) fn finish(mut self) -> Vec<PathCommand> {
        self.end_figure(false);
        self.commands
    }
}

/// The segments left for a document ran out.
pub(crate) struct OutOfSegments;

/// Takes one segment of the budget of the document.
fn spend(budget: &mut usize) -> Result<(), OutOfSegments> {
    *budget = budget.checked_sub(1).ok_or(OutOfSegments)?;
    Ok(())
}

/// The commands of path data. As SVG asks, a path with an error is drawn up to the error.
pub(crate) fn parse_path(text: &str, fill_rule: FillRule, budget: &mut usize) -> Result<Vec<PathCommand>, OutOfSegments> {
    let mut builder = PathBuilder::new(fill_rule);
    let mut scanner = Scanner::new(text);
    let mut command = 0u8;
    let mut previous = 0u8;
    let mut control = Point::new(0.0, 0.0);
    loop {
        scanner.skip_separator();
        let Some(next) = scanner.peek() else { break };
        if next.is_ascii_alphabetic() {
            command = next;
            scanner.advance();
            scanner.skip_whitespace();
        } else {
            // The arguments of another command of the same kind; after a move, of a line.
            command = match command {
                b'M' => b'L',
                b'm' => b'l',
                0 | b'Z' | b'z' => break,
                other => other,
            };
        }
        if previous == 0 && !matches!(command, b'M' | b'm') {
            break;
        }
        spend(budget)?;
        let is_relative = command.is_ascii_lowercase();
        let origin = if is_relative { builder.current } else { Point::new(0.0, 0.0) };
        let drawn = match command.to_ascii_uppercase() {
            b'M' => read_point(&mut scanner, origin).map(|point| builder.move_to(point)),
            b'L' => read_point(&mut scanner, origin).map(|point| builder.line_to(point)),
            b'H' => scanner.number().map(|x| builder.line_to(Point::new(x + origin.x, builder.current.y))),
            b'V' => scanner.number().map(|y| builder.line_to(Point::new(builder.current.x, y + origin.y))),
            b'C' => read_points::<3>(&mut scanner, origin).map(|[first, second, point]| {
                builder.cubic_to(first, second, point);
                control = second;
            }),
            b'S' => read_points::<2>(&mut scanner, origin).map(|[second, point]| {
                let first =
                    if matches!(previous.to_ascii_uppercase(), b'C' | b'S') { reflect(control, builder.current) } else { builder.current };
                builder.cubic_to(first, second, point);
                control = second;
            }),
            b'Q' => read_points::<2>(&mut scanner, origin).map(|[middle, point]| {
                builder.quadratic_to(middle, point);
                control = middle;
            }),
            b'T' => read_point(&mut scanner, origin).map(|point| {
                let middle =
                    if matches!(previous.to_ascii_uppercase(), b'Q' | b'T') { reflect(control, builder.current) } else { builder.current };
                builder.quadratic_to(middle, point);
                control = middle;
            }),
            b'A' => read_arc(&mut scanner, origin).map(|(radii, rotation, is_large, is_sweep, point)| {
                builder.arc_to(radii, rotation, is_large, is_sweep, point);
            }),
            b'Z' => {
                builder.close();
                Some(())
            }
            _ => None,
        };
        if drawn.is_none() {
            break;
        }
        previous = command;
    }
    Ok(builder.finish())
}

fn reflect(control: Point, about: Point) -> Point {
    Point::new(2.0 * about.x - control.x, 2.0 * about.y - control.y)
}

fn read_point(scanner: &mut Scanner<'_>, origin: Point) -> Option<Point> {
    let x = scanner.number()?;
    scanner.skip_separator();
    let y = scanner.number()?;
    Some(Point::new(x + origin.x, y + origin.y))
}

fn read_points<const N: usize>(scanner: &mut Scanner<'_>, origin: Point) -> Option<[Point; N]> {
    let mut points = [Point::new(0.0, 0.0); N];
    for (index, point) in points.iter_mut().enumerate() {
        if index > 0 {
            scanner.skip_separator();
        }
        *point = read_point(scanner, origin)?;
    }
    Some(points)
}

type Arc = ((f64, f64), f64, bool, bool, Point);

fn read_arc(scanner: &mut Scanner<'_>, origin: Point) -> Option<Arc> {
    let rx = scanner.number()?;
    scanner.skip_separator();
    let ry = scanner.number()?;
    scanner.skip_separator();
    let rotation = scanner.number()?;
    scanner.skip_separator();
    let is_large = scanner.flag()?;
    scanner.skip_separator();
    let is_sweep = scanner.flag()?;
    scanner.skip_separator();
    let point = read_point(scanner, origin)?;
    Some(((rx, ry), rotation, is_large, is_sweep, point))
}

/// The commands of a rectangle with corners of the radii `(rx, ry)`.
pub(crate) fn rectangle(rect: Rect, (rx, ry): (f64, f64), fill_rule: FillRule) -> Vec<PathCommand> {
    let mut builder = PathBuilder::new(fill_rule);
    let (left, top, right, bottom) = (rect.x, rect.y, rect.x + rect.width, rect.y + rect.height);
    if rx > 0.0 && ry > 0.0 {
        builder.move_to(Point::new(left + rx, top));
        builder.line_to(Point::new(right - rx, top));
        builder.arc_to((rx, ry), 0.0, false, true, Point::new(right, top + ry));
        builder.line_to(Point::new(right, bottom - ry));
        builder.arc_to((rx, ry), 0.0, false, true, Point::new(right - rx, bottom));
        builder.line_to(Point::new(left + rx, bottom));
        builder.arc_to((rx, ry), 0.0, false, true, Point::new(left, bottom - ry));
        builder.line_to(Point::new(left, top + ry));
        builder.arc_to((rx, ry), 0.0, false, true, Point::new(left + rx, top));
    } else {
        builder.move_to(Point::new(left, top));
        builder.line_to(Point::new(right, top));
        builder.line_to(Point::new(right, bottom));
        builder.line_to(Point::new(left, bottom));
    }
    builder.close();
    builder.finish()
}

/// The commands of an ellipse.
pub(crate) fn ellipse(center: Point, (rx, ry): (f64, f64), fill_rule: FillRule) -> Vec<PathCommand> {
    let mut builder = PathBuilder::new(fill_rule);
    builder.move_to(Point::new(center.x + rx, center.y));
    builder.arc_to((rx, ry), 0.0, false, true, Point::new(center.x, center.y + ry));
    builder.arc_to((rx, ry), 0.0, false, true, Point::new(center.x - rx, center.y));
    builder.arc_to((rx, ry), 0.0, false, true, Point::new(center.x, center.y - ry));
    builder.arc_to((rx, ry), 0.0, false, true, Point::new(center.x + rx, center.y));
    builder.close();
    builder.finish()
}

/// The commands of a line through `points`, closed for a polygon.
pub(crate) fn polyline(points: &[f64], is_closed: bool, fill_rule: FillRule) -> Vec<PathCommand> {
    let mut builder = PathBuilder::new(fill_rule);
    let mut pairs = points.chunks_exact(2).map(|pair| Point::new(pair[0], pair[1]));
    if let Some(first) = pairs.next() {
        builder.move_to(first);
        for point in pairs {
            builder.line_to(point);
        }
        if is_closed {
            builder.close();
        }
    }
    builder.finish()
}

/// Applies `matrix` to every point of `commands`.
pub(crate) fn transform_commands(commands: &mut [PathCommand], matrix: Matrix) {
    for command in commands {
        match command {
            PathCommand::BeginFigure(point, _) | PathCommand::LineTo(point, _) => *point = matrix.transform(*point),
            PathCommand::QuadraticBezierTo(control, point, _) => {
                *control = matrix.transform(*control);
                *point = matrix.transform(*point);
            }
            PathCommand::CubicBezierTo(first, second, point, _) => {
                *first = matrix.transform(*first);
                *second = matrix.transform(*second);
                *point = matrix.transform(*point);
            }
            // The decoder writes no arcs.
            PathCommand::ArcTo(..) | PathCommand::SetFillRule(_) | PathCommand::EndFigure(_) => {}
        }
    }
}

/// Whether every coordinate of `commands` is finite.
pub(crate) fn is_finite(commands: &[PathCommand]) -> bool {
    let finite = |point: &Point| point.x.is_finite() && point.y.is_finite();
    commands.iter().all(|command| match command {
        PathCommand::BeginFigure(point, _) | PathCommand::LineTo(point, _) => finite(point),
        PathCommand::QuadraticBezierTo(control, point, _) => finite(control) && finite(point),
        PathCommand::CubicBezierTo(first, second, point, _) => finite(first) && finite(second) && finite(point),
        PathCommand::ArcTo(..) | PathCommand::SetFillRule(_) | PathCommand::EndFigure(_) => true,
    })
}

/// The extent along one axis of a curve with these control values: the ends and where its
/// derivative is zero.
fn curve_extent(values: &[f64], bounds: &mut (f64, f64)) {
    let mut include = |value: f64| {
        bounds.0 = bounds.0.min(value);
        bounds.1 = bounds.1.max(value);
    };
    include(values[0]);
    include(values[values.len() - 1]);
    let mut at = |t: f64| {
        if t > 0.0 && t < 1.0 {
            let mut points = values.to_vec();
            while points.len() > 1 {
                for index in 0..points.len() - 1 {
                    points[index] += (points[index + 1] - points[index]) * t;
                }
                points.pop();
            }
            include(points[0]);
        }
    };
    match *values {
        [a, b, c] => {
            let denominator = a - 2.0 * b + c;
            if denominator != 0.0 {
                at((a - b) / denominator);
            }
        }
        [p0, p1, p2, p3] => {
            let (a, b, c) = (p3 - p0 + 3.0 * (p1 - p2), 2.0 * (p0 - 2.0 * p1 + p2), p1 - p0);
            if a.abs() < 1e-12 {
                if b != 0.0 {
                    at(-c / b);
                }
            } else {
                let discriminant = b * b - 4.0 * a * c;
                if discriminant >= 0.0 {
                    let root = discriminant.sqrt();
                    at((-b + root) / (2.0 * a));
                    at((-b - root) / (2.0 * a));
                }
            }
        }
        _ => {}
    }
}

/// The bounding box of the figures of `commands`, without stroke.
pub(crate) fn bounds(commands: &[PathCommand]) -> Option<Rect> {
    let (mut xs, mut ys) = ((f64::INFINITY, f64::NEG_INFINITY), (f64::INFINITY, f64::NEG_INFINITY));
    let mut current = Point::new(0.0, 0.0);
    let mut any = false;
    for command in commands {
        match *command {
            PathCommand::BeginFigure(point, _) | PathCommand::LineTo(point, _) => {
                curve_extent(&[point.x, point.x], &mut xs);
                curve_extent(&[point.y, point.y], &mut ys);
                current = point;
                any = true;
            }
            PathCommand::QuadraticBezierTo(control, point, _) => {
                curve_extent(&[current.x, control.x, point.x], &mut xs);
                curve_extent(&[current.y, control.y, point.y], &mut ys);
                current = point;
            }
            PathCommand::CubicBezierTo(first, second, point, _) => {
                curve_extent(&[current.x, first.x, second.x, point.x], &mut xs);
                curve_extent(&[current.y, first.y, second.y, point.y], &mut ys);
                current = point;
            }
            PathCommand::ArcTo(..) | PathCommand::SetFillRule(_) | PathCommand::EndFigure(_) => {}
        }
    }
    any.then(|| Rect::new(xs.0, ys.0, xs.1 - xs.0, ys.1 - ys.0))
}
