//! Paths of cubic segments: Lottie Bezier shapes, the primitive shapes, their transforms, and trim
//! paths by arc length.

use ferroui_base::{Matrix, Point};
use serde_json::Value;

/// A command of a path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCommand {
    MoveTo(Point),
    LineTo(Point),
    CubicTo(Point, Point, Point),
    Close,
}

/// A path in the space of the composition or of a shape.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PathData {
    pub commands: Vec<PathCommand>,
}

/// The control point distance that makes a cubic Bezier a quarter circle of radius one.
const CIRCLE: f64 = 0.552_284_749_830_793_6;

impl PathData {
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub(crate) fn append(&mut self, other: &PathData) {
        self.commands.extend_from_slice(&other.commands);
    }

    pub(crate) fn transformed(&self, matrix: Matrix) -> PathData {
        let at = |point: Point| matrix.transform(point);
        PathData {
            commands: self
                .commands
                .iter()
                .map(|command| match *command {
                    PathCommand::MoveTo(p) => PathCommand::MoveTo(at(p)),
                    PathCommand::LineTo(p) => PathCommand::LineTo(at(p)),
                    PathCommand::CubicTo(a, b, c) => PathCommand::CubicTo(at(a), at(b), at(c)),
                    PathCommand::Close => PathCommand::Close,
                })
                .collect(),
        }
    }

    /// A rectangle centred on `center`, its corners rounded by `radius`, clockwise from the top
    /// right, as Lottie winds it.
    pub(crate) fn rectangle(center: Point, width: f64, height: f64, radius: f64) -> PathData {
        let (hw, hh) = (width * 0.5, height * 0.5);
        let r = radius.max(0.0).min(hw).min(hh);
        let (l, t, right, b) = (center.x - hw, center.y - hh, center.x + hw, center.y + hh);
        let p = Point::new;
        if r <= 0.0 {
            return PathData {
                commands: vec![
                    PathCommand::MoveTo(p(right, t)),
                    PathCommand::LineTo(p(right, b)),
                    PathCommand::LineTo(p(l, b)),
                    PathCommand::LineTo(p(l, t)),
                    PathCommand::Close,
                ],
            };
        }
        let k = r * (1.0 - CIRCLE);
        PathData {
            commands: vec![
                PathCommand::MoveTo(p(right, t + r)),
                PathCommand::LineTo(p(right, b - r)),
                PathCommand::CubicTo(p(right, b - k), p(right - k, b), p(right - r, b)),
                PathCommand::LineTo(p(l + r, b)),
                PathCommand::CubicTo(p(l + k, b), p(l, b - k), p(l, b - r)),
                PathCommand::LineTo(p(l, t + r)),
                PathCommand::CubicTo(p(l, t + k), p(l + k, t), p(l + r, t)),
                PathCommand::LineTo(p(right - r, t)),
                PathCommand::CubicTo(p(right - k, t), p(right, t + k), p(right, t + r)),
                PathCommand::Close,
            ],
        }
    }

    /// An ellipse centred on `center`, clockwise from the top.
    pub(crate) fn ellipse(center: Point, width: f64, height: f64) -> PathData {
        let (rx, ry) = (width * 0.5, height * 0.5);
        let (kx, ky) = (rx * CIRCLE, ry * CIRCLE);
        let (x, y) = (center.x, center.y);
        let p = Point::new;
        PathData {
            commands: vec![
                PathCommand::MoveTo(p(x, y - ry)),
                PathCommand::CubicTo(p(x + kx, y - ry), p(x + rx, y - ky), p(x + rx, y)),
                PathCommand::CubicTo(p(x + rx, y + ky), p(x + kx, y + ry), p(x, y + ry)),
                PathCommand::CubicTo(p(x - kx, y + ry), p(x - rx, y + ky), p(x - rx, y)),
                PathCommand::CubicTo(p(x - rx, y - ky), p(x - kx, y - ry), p(x, y - ry)),
                PathCommand::Close,
            ],
        }
    }

    /// The segments of each contour, every one as a cubic.
    fn contours(&self) -> Vec<Vec<[Point; 4]>> {
        let mut contours = Vec::new();
        let mut current: Vec<[Point; 4]> = Vec::new();
        let (mut start, mut last) = (Point::new(0.0, 0.0), Point::new(0.0, 0.0));
        let line = |a: Point, b: Point| {
            [a, Point::new(a.x + (b.x - a.x) / 3.0, a.y + (b.y - a.y) / 3.0), Point::new(a.x + (b.x - a.x) * 2.0 / 3.0, a.y + (b.y - a.y) * 2.0 / 3.0), b]
        };
        for command in &self.commands {
            match *command {
                PathCommand::MoveTo(point) => {
                    if !current.is_empty() {
                        contours.push(std::mem::take(&mut current));
                    }
                    start = point;
                    last = point;
                }
                PathCommand::LineTo(point) => {
                    current.push(line(last, point));
                    last = point;
                }
                PathCommand::CubicTo(a, b, c) => {
                    current.push([last, a, b, c]);
                    last = c;
                }
                PathCommand::Close => {
                    if last != start {
                        current.push(line(last, start));
                    }
                    last = start;
                }
            }
        }
        if !current.is_empty() {
            contours.push(current);
        }
        contours
    }

    /// The part of the path from `start` to `end` (fractions of its length, `end` may be less than
    /// `start`, which goes through the end of the path).
    pub(crate) fn trimmed(&self, start: f64, end: f64) -> PathData {
        let segments: Vec<[Point; 4]> = self.contours().into_iter().flatten().collect();
        let lengths: Vec<f64> = segments.iter().map(segment_length).collect();
        let total: f64 = lengths.iter().sum();
        let mut result = PathData::default();
        if total <= 0.0 {
            return result;
        }
        let mut extract = |from: f64, to: f64| {
            let (from, to) = (from * total, to * total);
            let (mut position, mut open) = (0.0, false);
            for (segment, length) in segments.iter().zip(&lengths) {
                let (a, b) = (position, position + length);
                position = b;
                if *length <= 0.0 || b <= from || a >= to {
                    open = false;
                    continue;
                }
                let t0 = ((from - a) / length).clamp(0.0, 1.0);
                let t1 = ((to - a) / length).clamp(0.0, 1.0);
                let piece = sub_segment(segment, t0, t1);
                if !open || t0 > 0.0 {
                    result.commands.push(PathCommand::MoveTo(piece[0]));
                }
                result.commands.push(PathCommand::CubicTo(piece[1], piece[2], piece[3]));
                open = true;
            }
        };
        if end >= start {
            extract(start, end);
        } else {
            extract(start, 1.0);
            extract(0.0, end);
        }
        result
    }

    /// The length of the path.
    #[cfg(test)]
    pub(crate) fn length(&self) -> f64 {
        self.contours().iter().flatten().map(segment_length).sum()
    }
}

fn segment_at(segment: &[Point; 4], t: f64) -> Point {
    let u = 1.0 - t;
    let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
    Point::new(
        w[0] * segment[0].x + w[1] * segment[1].x + w[2] * segment[2].x + w[3] * segment[3].x,
        w[0] * segment[0].y + w[1] * segment[1].y + w[2] * segment[2].y + w[3] * segment[3].y,
    )
}

fn segment_length(segment: &[Point; 4]) -> f64 {
    const STEPS: usize = 16;
    let mut last = segment[0];
    let mut length = 0.0;
    for step in 1..=STEPS {
        let point = segment_at(segment, step as f64 / STEPS as f64);
        length += (point.x - last.x).hypot(point.y - last.y);
        last = point;
    }
    length
}

fn mix(a: Point, b: Point, t: f64) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

/// The part of a cubic between `t0` and `t1`.
fn sub_segment(s: &[Point; 4], t0: f64, t1: f64) -> [Point; 4] {
    // Split at t1 and keep the first part.
    let (a, b, c) = (mix(s[0], s[1], t1), mix(s[1], s[2], t1), mix(s[2], s[3], t1));
    let (d, e) = (mix(a, b, t1), mix(b, c, t1));
    let f = mix(d, e, t1);
    let first = [s[0], a, d, f];
    if t0 <= 0.0 || t1 <= 0.0 {
        return first;
    }
    // Split that at t0, in its own parameter, and keep the second part.
    let t = t0 / t1;
    let (a, b, c) = (mix(first[0], first[1], t), mix(first[1], first[2], t), mix(first[2], first[3], t));
    let (d, e) = (mix(a, b, t), mix(b, c, t));
    [mix(d, e, t), e, c, first[3]]
}

/// A Bezier shape of Lottie (`v`, `i`, `o`, `c`): vertices with tangents relative to them.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct BezierShape {
    closed: bool,
    vertices: Vec<[f64; 2]>,
    tangents_in: Vec<[f64; 2]>,
    tangents_out: Vec<[f64; 2]>,
}

fn points(value: Option<&Value>) -> Option<Vec<[f64; 2]>> {
    value?
        .as_array()?
        .iter()
        .map(|item| {
            let item = item.as_array()?;
            Some([item.first()?.as_f64()?, item.get(1)?.as_f64()?])
        })
        .collect()
}

impl BezierShape {
    pub(crate) fn parse(value: &Value) -> Option<Self> {
        // Keyframed shapes hold their value in an array of one.
        let value = value.as_array().and_then(|items| items.first()).unwrap_or(value);
        let vertices = points(value.get("v"))?;
        let tangents_in = points(value.get("i"))?;
        let tangents_out = points(value.get("o"))?;
        if tangents_in.len() != vertices.len() || tangents_out.len() != vertices.len() {
            return None;
        }
        Some(Self { closed: value.get("c").and_then(Value::as_bool).unwrap_or(false), vertices, tangents_in, tangents_out })
    }

    /// This shape towards `other` by `t`; shapes of different vertex counts do not interpolate.
    pub(crate) fn lerp(&self, other: &Self, t: f64) -> Self {
        if self.vertices.len() != other.vertices.len() {
            return self.clone();
        }
        let mix = |a: &[[f64; 2]], b: &[[f64; 2]]| -> Vec<[f64; 2]> {
            a.iter().zip(b).map(|(a, b)| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]).collect()
        };
        Self {
            closed: self.closed,
            vertices: mix(&self.vertices, &other.vertices),
            tangents_in: mix(&self.tangents_in, &other.tangents_in),
            tangents_out: mix(&self.tangents_out, &other.tangents_out),
        }
    }

    pub(crate) fn to_path(&self) -> PathData {
        let count = self.vertices.len();
        let mut commands = Vec::with_capacity(count + 2);
        let Some(first) = self.vertices.first() else {
            return PathData::default();
        };
        let p = |v: [f64; 2]| Point::new(v[0], v[1]);
        commands.push(PathCommand::MoveTo(p(*first)));
        let segments = if self.closed { count } else { count.saturating_sub(1) };
        for index in 0..segments {
            let next = (index + 1) % count;
            let (a, b) = (self.vertices[index], self.vertices[next]);
            let (out, into) = (self.tangents_out[index], self.tangents_in[next]);
            if out == [0.0; 2] && into == [0.0; 2] {
                commands.push(PathCommand::LineTo(p(b)));
            } else {
                commands.push(PathCommand::CubicTo(
                    p([a[0] + out[0], a[1] + out[1]]),
                    p([b[0] + into[0], b[1] + into[1]]),
                    p(b),
                ));
            }
        }
        if self.closed {
            commands.push(PathCommand::Close);
        }
        PathData { commands }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_bezier_shape_becomes_lines_and_cubics() {
        let shape = BezierShape::parse(&json!({
            "c": true, "v": [[0, 0], [10, 0], [10, 10]], "i": [[0, 0], [0, 0], [0, -5]], "o": [[0, 0], [0, 5], [0, 0]]
        }))
        .unwrap();
        let path = shape.to_path();
        assert_eq!(path.commands.len(), 5);
        assert!(matches!(path.commands[1], PathCommand::LineTo(_)));
        assert!(matches!(path.commands[2], PathCommand::CubicTo(..)));
        assert_eq!(path.commands[4], PathCommand::Close);
    }

    #[test]
    fn trimming_keeps_the_fraction_of_the_length() {
        let square = PathData::rectangle(Point::new(5.0, 5.0), 10.0, 10.0, 0.0);
        assert!((square.length() - 40.0).abs() < 1e-6);
        assert!((square.trimmed(0.0, 0.25).length() - 10.0).abs() < 1e-6);
        assert!((square.trimmed(0.1, 0.6).length() - 20.0).abs() < 1e-6);
        // Through the end of the path.
        assert!((square.trimmed(0.75, 0.25).length() - 20.0).abs() < 1e-6);
        assert!(square.trimmed(0.5, 0.5).is_empty());
    }

    #[test]
    fn an_ellipse_has_the_circumference_of_a_circle() {
        let circle = PathData::ellipse(Point::new(0.0, 0.0), 20.0, 20.0);
        assert!((circle.length() - std::f64::consts::TAU * 10.0).abs() < 0.05, "{}", circle.length());
    }

    #[test]
    fn a_rounded_rectangle_is_shorter_than_its_corners() {
        let plain = PathData::rectangle(Point::new(0.0, 0.0), 20.0, 20.0, 0.0).length();
        let round = PathData::rectangle(Point::new(0.0, 0.0), 20.0, 20.0, 5.0).length();
        assert!(round < plain);
    }
}
