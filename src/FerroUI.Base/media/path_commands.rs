//! Path data recorded when the markup is compiled (#603, docs/xaml-themes.md 4.6).
//!
//! Not a port of an upstream file. Upstream parses the path data of a document
//! (`Data="M 0 0 L 10 10"`) when the document is loaded, with `StreamGeometry.Parse`. The
//! compiler of FerroUI runs the same parser ([`PathMarkupParser`]) when it compiles the markup,
//! records the calls it makes on its geometry context as [`PathCommand`]s, and writes them as
//! a constant table; the generated code builds the geometry from the table
//! ([`StreamGeometry::from_commands`](super::StreamGeometry::from_commands)). The geometry is
//! the one the parser builds from the text: the same calls on the same context, in the same
//! order. Path data the parser refuses is left to the run-time parse, which reports the error
//! as the load error it is.

use crate::media::{FillRule, PathMarkupParser, SweepDirection};
use crate::platform::IGeometryContext;
use crate::utilities::FormatError;
use crate::{Point, Size};

/// One call of the path markup parser on its geometry context.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCommand {
    /// `set_fill_rule(fill_rule)`.
    SetFillRule(FillRule),
    /// `begin_figure(start_point, is_filled)`.
    BeginFigure(Point, bool),
    /// `line_to(point, is_stroked)`.
    LineTo(Point, bool),
    /// `quadratic_bezier_to(control_point, end_point, is_stroked)`.
    QuadraticBezierTo(Point, Point, bool),
    /// `cubic_bezier_to(control_point1, control_point2, end_point, is_stroked)`.
    CubicBezierTo(Point, Point, Point, bool),
    /// `arc_to(point, size, rotation_angle, is_large_arc, sweep_direction, is_stroked)`.
    ArcTo(Point, Size, f64, bool, SweepDirection, bool),
    /// `end_figure(is_closed)`.
    EndFigure(bool),
}

impl PathCommand {
    /// The calls [`PathMarkupParser`] makes for `path_data`, in order; the error of the parser
    /// when it refuses the text.
    pub fn record(path_data: &str) -> Result<Vec<PathCommand>, FormatError> {
        let mut recorder = Recorder::default();
        {
            let mut parser = PathMarkupParser::new(&mut recorder);
            let result = parser.parse(path_data);
            parser.dispose();
            result?;
        }
        Ok(recorder.commands)
    }

    /// Makes the calls of `commands` on `context`, in order.
    pub fn replay(commands: &[PathCommand], context: &mut dyn IGeometryContext) {
        for command in commands {
            match *command {
                PathCommand::SetFillRule(fill_rule) => context.set_fill_rule(fill_rule),
                PathCommand::BeginFigure(start_point, is_filled) => context.begin_figure(start_point, is_filled),
                PathCommand::LineTo(point, is_stroked) => context.line_to(point, is_stroked),
                PathCommand::QuadraticBezierTo(control_point, end_point, is_stroked) => {
                    context.quadratic_bezier_to(control_point, end_point, is_stroked)
                }
                PathCommand::CubicBezierTo(control_point1, control_point2, end_point, is_stroked) => {
                    context.cubic_bezier_to(control_point1, control_point2, end_point, is_stroked)
                }
                PathCommand::ArcTo(point, size, rotation_angle, is_large_arc, sweep_direction, is_stroked) => {
                    context.arc_to(point, size, rotation_angle, is_large_arc, sweep_direction, is_stroked)
                }
                PathCommand::EndFigure(is_closed) => context.end_figure(is_closed),
            }
        }
    }

    /// Whether every number of the command is finite: what a constant table can state as
    /// Rust literals.
    pub fn is_finite(&self) -> bool {
        let point = |point: &Point| point.x.is_finite() && point.y.is_finite();
        match self {
            PathCommand::SetFillRule(_) | PathCommand::EndFigure(_) => true,
            PathCommand::BeginFigure(start_point, _) => point(start_point),
            PathCommand::LineTo(end_point, _) => point(end_point),
            PathCommand::QuadraticBezierTo(control_point, end_point, _) => point(control_point) && point(end_point),
            PathCommand::CubicBezierTo(control_point1, control_point2, end_point, _) => {
                point(control_point1) && point(control_point2) && point(end_point)
            }
            PathCommand::ArcTo(end_point, size, rotation_angle, ..) => {
                point(end_point) && size.width.is_finite() && size.height.is_finite() && rotation_angle.is_finite()
            }
        }
    }
}

/// The geometry context that records the calls of the parser.
#[derive(Default)]
struct Recorder {
    commands: Vec<PathCommand>,
}

impl IGeometryContext for Recorder {
    fn arc_to(
        &mut self,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
        is_stroked: bool,
    ) {
        self.commands.push(PathCommand::ArcTo(point, size, rotation_angle, is_large_arc, sweep_direction, is_stroked));
    }

    fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
        self.commands.push(PathCommand::BeginFigure(start_point, is_filled));
    }

    fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool) {
        self.commands.push(PathCommand::CubicBezierTo(control_point1, control_point2, end_point, is_stroked));
    }

    fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, is_stroked: bool) {
        self.commands.push(PathCommand::QuadraticBezierTo(control_point, end_point, is_stroked));
    }

    fn line_to(&mut self, point: Point, is_stroked: bool) {
        self.commands.push(PathCommand::LineTo(point, is_stroked));
    }

    fn end_figure(&mut self, is_closed: bool) {
        self.commands.push(PathCommand::EndFigure(is_closed));
    }

    fn set_fill_rule(&mut self, fill_rule: FillRule) {
        self.commands.push(PathCommand::SetFillRule(fill_rule));
    }

    fn dispose(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A context that writes every call as text, to compare the calls of the parser with the
    /// calls of the replay.
    #[derive(Default)]
    struct Transcript(Vec<String>);

    impl IGeometryContext for Transcript {
        fn arc_to(&mut self, point: Point, size: Size, rotation_angle: f64, is_large_arc: bool, sweep_direction: SweepDirection, is_stroked: bool) {
            self.0.push(format!("arc {point:?} {size:?} {rotation_angle:?} {is_large_arc} {sweep_direction:?} {is_stroked}"));
        }
        fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
            self.0.push(format!("begin {start_point:?} {is_filled}"));
        }
        fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool) {
            self.0.push(format!("cubic {control_point1:?} {control_point2:?} {end_point:?} {is_stroked}"));
        }
        fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, is_stroked: bool) {
            self.0.push(format!("quadratic {control_point:?} {end_point:?} {is_stroked}"));
        }
        fn line_to(&mut self, point: Point, is_stroked: bool) {
            self.0.push(format!("line {point:?} {is_stroked}"));
        }
        fn end_figure(&mut self, is_closed: bool) {
            self.0.push(format!("end {is_closed}"));
        }
        fn set_fill_rule(&mut self, fill_rule: FillRule) {
            self.0.push(format!("fill {fill_rule:?}"));
        }
        fn dispose(&mut self) {}
    }

    /// The replay of the recorded commands makes the calls the parser makes, for every command
    /// of the path markup (relative and absolute, smooth curves, arcs, the fill rule, closed
    /// and open figures).
    #[test]
    fn the_replay_makes_the_calls_of_the_parser() {
        for path_data in [
            "M 0 0 L 10 10",
            "F1 M10,20 L30,40 C1,2 3,4 5,6 Q1,2 3,4 A10,20 30 1 0 40,50 Z",
            "M5.5 10.586 1.707 6.793A1 1 0 0 0 .293 8.207l4.5 4.5a1 1 0 0 0 1.414 0l11-11A1 1 0 0 0 15.793.293L5.5 10.586Z",
            "M0,0 H10 V10 h-5 v-5 S1,2 3,4 s1,1 2,2 T5,5 t1,1 z M20,20 L30,30",
            "F0 M 80, 200 A 100, 50 45 1 0 100, 50",
        ] {
            let mut parsed = Transcript::default();
            PathMarkupParser::new(&mut parsed).parse(path_data).unwrap();
            let commands = PathCommand::record(path_data).unwrap();
            assert!(commands.iter().all(PathCommand::is_finite), "{path_data}");
            let mut replayed = Transcript::default();
            PathCommand::replay(&commands, &mut replayed);
            assert_eq!(replayed.0, parsed.0, "{path_data}");
        }
    }

    /// Text the parser refuses is not recorded: its error is the error of the parser.
    #[test]
    fn refused_path_data_is_the_error_of_the_parser() {
        let mut ignored = Transcript::default();
        let expected = PathMarkupParser::new(&mut ignored).parse("M 0 0 X 10").unwrap_err();
        assert_eq!(PathCommand::record("M 0 0 X 10").unwrap_err(), expected);
    }

    #[test]
    fn a_number_that_is_not_finite_is_reported() {
        assert!(!PathCommand::LineTo(Point::new(f64::NAN, 0.0), true).is_finite());
        assert!(!PathCommand::ArcTo(Point::new(0.0, 0.0), Size::new(f64::INFINITY, 1.0), 0.0, false, SweepDirection::Clockwise, true).is_finite());
        assert!(PathCommand::EndFigure(true).is_finite());
    }
}
