use crate::media::{FillRule, SweepDirection};
use crate::platform::IGeometryContext;
use crate::utilities::span_helpers::parse_double;
use crate::utilities::FormatError;
use crate::{Point, Size};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    #[allow(dead_code)]
    None,
    FillRule,
    Move,
    Line,
    HorizontalLine,
    VerticalLine,
    CubicBezierCurve,
    QuadraticBezierCurve,
    SmoothCubicBezierCurve,
    SmoothQuadraticBezierCurve,
    Arc,
    Close,
}

fn command_from_char(c: char) -> Option<Command> {
    Some(match c {
        'F' => Command::FillRule,
        'M' => Command::Move,
        'L' => Command::Line,
        'H' => Command::HorizontalLine,
        'V' => Command::VerticalLine,
        'Q' => Command::QuadraticBezierCurve,
        'T' => Command::SmoothQuadraticBezierCurve,
        'C' => Command::CubicBezierCurve,
        'S' => Command::SmoothCubicBezierCurve,
        'A' => Command::Arc,
        'Z' => Command::Close,
        _ => return None,
    })
}

/// Parses a path markup string.
pub struct PathMarkupParser<'a> {
    geometry_context: Option<&'a mut dyn IGeometryContext>,
    current_point: Point,
    begin_figure_point: Option<Point>,
    previous_control_point: Option<Point>,
    is_open: bool,
    is_disposed: bool,
}

impl<'a> PathMarkupParser<'a> {
    /// Creates a parser that draws into `geometry_context`.
    pub fn new(geometry_context: &'a mut dyn IGeometryContext) -> Self {
        Self {
            geometry_context: Some(geometry_context),
            current_point: Point::default(),
            begin_figure_point: None,
            previous_control_point: None,
            is_open: false,
            is_disposed: false,
        }
    }

    /// Releases the geometry context. Using the parser afterwards panics.
    pub fn dispose(&mut self) {
        if self.is_disposed {
            return;
        }

        self.geometry_context = None;
        self.is_disposed = true;
    }

    fn context(&mut self) -> &mut dyn IGeometryContext {
        match self.geometry_context.as_deref_mut() {
            Some(context) if !self.is_disposed => context,
            _ => panic!("PathMarkupParser is disposed"),
        }
    }

    fn mirror_control_point(control_point: Point, center: Point) -> Point {
        let dir = control_point - center;
        center + -dir
    }

    /// Parses the specified path data and writes the result to the geometry
    /// context of this instance.
    pub fn parse(&mut self, path_data: &str) -> Result<(), FormatError> {
        let _ = self.context();

        let mut span = path_data;
        self.current_point = Point::default();

        while !span.is_empty() {
            let Some((command, relative)) = read_command(&mut span)? else {
                break;
            };

            let mut initial_command = true;

            loop {
                if !initial_command {
                    span = read_separator(span);
                }

                match command {
                    Command::None => {}
                    Command::FillRule => self.set_fill_rule(&mut span)?,
                    Command::Move => self.add_move(&mut span, relative)?,
                    Command::Line => self.add_line(&mut span, relative)?,
                    Command::HorizontalLine => self.add_horizontal_line(&mut span, relative)?,
                    Command::VerticalLine => self.add_vertical_line(&mut span, relative)?,
                    Command::CubicBezierCurve => self.add_cubic_bezier_curve(&mut span, relative)?,
                    Command::QuadraticBezierCurve => self.add_quadratic_bezier_curve(&mut span, relative)?,
                    Command::SmoothCubicBezierCurve => self.add_smooth_cubic_bezier_curve(&mut span, relative)?,
                    Command::SmoothQuadraticBezierCurve => {
                        self.add_smooth_quadratic_bezier_curve(&mut span, relative)?
                    }
                    Command::Arc => self.add_arc(&mut span, relative)?,
                    Command::Close => self.close_figure(),
                }

                initial_command = false;

                if !peek_argument(span) {
                    break;
                }
            }
        }

        if self.is_open {
            self.context().end_figure(false);
        }

        Ok(())
    }

    fn create_figure(&mut self) {
        if self.is_open {
            self.context().end_figure(false);
        }

        let current_point = self.current_point;
        self.context().begin_figure(current_point, true);

        self.begin_figure_point = Some(current_point);

        self.is_open = true;
    }

    fn set_fill_rule(&mut self, span: &mut &str) -> Result<(), FormatError> {
        let fill_rule = match read_argument(span) {
            Some(fill_rule) if fill_rule.len() == 1 => fill_rule,
            _ => return Err(FormatError::new("Invalid fill rule.")),
        };

        let rule = match fill_rule {
            "0" => FillRule::EvenOdd,
            "1" => FillRule::NonZero,
            _ => return Err(FormatError::new("Invalid fill rule")),
        };

        self.context().set_fill_rule(rule);
        Ok(())
    }

    fn close_figure(&mut self) {
        if self.is_open {
            self.context().end_figure(true);
            if let Some(begin_figure_point) = self.begin_figure_point.take() {
                self.current_point = begin_figure_point;
            }
        }

        self.previous_control_point = None;

        self.is_open = false;
    }

    fn add_move(&mut self, span: &mut &str, relative: bool) -> Result<(), FormatError> {
        let current_point =
            if relative { read_relative_point(span, self.current_point)? } else { read_point(span)? };

        self.current_point = current_point;

        self.create_figure();

        while peek_argument(span) {
            *span = read_separator(span);

            self.add_line(span, relative)?;
        }

        Ok(())
    }

    fn add_line(&mut self, span: &mut &str, relative: bool) -> Result<(), FormatError> {
        let next = if relative { read_relative_point(span, self.current_point)? } else { read_point(span)? };

        if !self.is_open {
            self.create_figure();
        }

        self.context().line_to(next, true);

        self.current_point = next;
        Ok(())
    }

    fn add_horizontal_line(&mut self, span: &mut &str, relative: bool) -> Result<(), FormatError> {
        let value = read_double(span)?;
        let next = if relative {
            Point::new(self.current_point.x + value, self.current_point.y)
        } else {
            self.current_point.with_x(value)
        };

        if !self.is_open {
            self.create_figure();
        }

        self.context().line_to(next, true);

        self.current_point = next;
        Ok(())
    }

    fn add_vertical_line(&mut self, span: &mut &str, relative: bool) -> Result<(), FormatError> {
        let value = read_double(span)?;
        let next = if relative {
            Point::new(self.current_point.x, self.current_point.y + value)
        } else {
            self.current_point.with_y(value)
        };

        if !self.is_open {
            self.create_figure();
        }

        self.context().line_to(next, true);

        self.current_point = next;
        Ok(())
    }

    fn read_command_point(&self, span: &mut &str, relative: bool) -> Result<Point, FormatError> {
        if relative {
            read_relative_point(span, self.current_point)
        } else {
            read_point(span)
        }
    }

    fn add_cubic_bezier_curve(&mut self, span: &mut &str, relative: bool) -> Result<(), FormatError> {
        let point1 = self.read_command_point(span, relative)?;

        *span = read_separator(span);

        let point2 = self.read_command_point(span, relative)?;

        self.previous_control_point = Some(point2);

        *span = read_separator(span);

        let point3 = self.read_command_point(span, relative)?;

        if !self.is_open {
            self.create_figure();
        }

        self.context().cubic_bezier_to(point1, point2, point3, true);

        self.current_point = point3;
        Ok(())
    }

    fn add_quadratic_bezier_curve(&mut self, span: &mut &str, relative: bool) -> Result<(), FormatError> {
        let start = self.read_command_point(span, relative)?;

        self.previous_control_point = Some(start);

        *span = read_separator(span);

        let end = self.read_command_point(span, relative)?;

        if !self.is_open {
            self.create_figure();
        }

        self.context().quadratic_bezier_to(start, end, true);

        self.current_point = end;
        Ok(())
    }

    fn add_smooth_cubic_bezier_curve(&mut self, span: &mut &str, relative: bool) -> Result<(), FormatError> {
        let point2 = self.read_command_point(span, relative)?;

        *span = read_separator(span);

        let end = self.read_command_point(span, relative)?;

        if let Some(previous_control_point) = self.previous_control_point {
            self.previous_control_point = Some(Self::mirror_control_point(previous_control_point, self.current_point));
        }

        if !self.is_open {
            self.create_figure();
        }

        let point1 = self.previous_control_point.unwrap_or(self.current_point);
        self.context().cubic_bezier_to(point1, point2, end, true);

        self.previous_control_point = Some(point2);

        self.current_point = end;
        Ok(())
    }

    fn add_smooth_quadratic_bezier_curve(&mut self, span: &mut &str, relative: bool) -> Result<(), FormatError> {
        let end = self.read_command_point(span, relative)?;

        if let Some(previous_control_point) = self.previous_control_point {
            self.previous_control_point = Some(Self::mirror_control_point(previous_control_point, self.current_point));
        }

        if !self.is_open {
            self.create_figure();
        }

        let control_point = self.previous_control_point.unwrap_or(self.current_point);
        self.context().quadratic_bezier_to(control_point, end, true);

        self.current_point = end;
        Ok(())
    }

    fn add_arc(&mut self, span: &mut &str, relative: bool) -> Result<(), FormatError> {
        let size = read_size(span)?;

        *span = read_separator(span);

        let rotation_angle = read_double(span)?;
        *span = read_separator(span);
        let is_large_arc = read_bool(span)?;

        *span = read_separator(span);

        let sweep_direction =
            if read_bool(span)? { SweepDirection::Clockwise } else { SweepDirection::CounterClockwise };

        *span = read_separator(span);

        let end = self.read_command_point(span, relative)?;

        if !self.is_open {
            self.create_figure();
        }

        self.context().arc_to(end, size, rotation_angle, is_large_arc, sweep_direction, true);

        self.current_point = end;

        self.previous_control_point = None;
        Ok(())
    }
}

fn peek_argument(span: &str) -> bool {
    let span = skip_whitespace(span);

    match span.chars().next() {
        Some(c) => c == ',' || c == '-' || c == '.' || c.is_ascii_digit(),
        None => false,
    }
}

/// Reads one numeric argument off the front of `remaining`. All inspected
/// characters are ASCII, so the byte offsets are character boundaries.
fn read_argument<'s>(remaining: &mut &'s str) -> Option<&'s str> {
    *remaining = skip_whitespace(remaining);
    if remaining.is_empty() {
        return None;
    }

    let bytes = remaining.as_bytes();
    let mut valid = false;
    let mut i = 0;
    if bytes[i] == b'-' {
        i += 1;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        valid = true;
        i += 1;
    }

    if i < bytes.len() && bytes[i] == b'.' {
        valid = false;
        i += 1;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        valid = true;
        i += 1;
    }

    if i < bytes.len() {
        // scientific notation
        if bytes[i] == b'E' || bytes[i] == b'e' {
            valid = false;
            i += 1;
            if i < bytes.len() && (bytes[i] == b'-' || bytes[i] == b'+') {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    valid = true;
                    i += 1;
                }
            }
        }
    }

    if !valid {
        return None;
    }
    let (argument, rest) = remaining.split_at(i);
    *remaining = rest;
    Some(argument)
}

fn read_separator(span: &str) -> &str {
    let span = skip_whitespace(span);
    span.strip_prefix(',').unwrap_or(span)
}

fn skip_whitespace(span: &str) -> &str {
    span.trim_start_matches(char::is_whitespace)
}

fn read_bool(span: &mut &str) -> Result<bool, FormatError> {
    *span = skip_whitespace(span);

    let mut chars = span.chars();
    let Some(c) = chars.next() else {
        return Err(FormatError::new("Invalid bool rule."));
    };

    *span = chars.as_str();

    match c {
        '0' => Ok(false),
        '1' => Ok(true),
        _ => Err(FormatError::new("Invalid bool rule")),
    }
}

fn read_double(span: &mut &str) -> Result<f64, FormatError> {
    let Some(double_value) = read_argument(span) else {
        return Err(FormatError::new("Invalid double value"));
    };

    parse_double(double_value).ok_or_else(|| FormatError::invalid_input(double_value))
}

fn read_size(span: &mut &str) -> Result<Size, FormatError> {
    let width = read_double(span)?;
    *span = read_separator(span);
    let height = read_double(span)?;
    Ok(Size::new(width, height))
}

fn read_point(span: &mut &str) -> Result<Point, FormatError> {
    let x = read_double(span)?;
    *span = read_separator(span);
    let y = read_double(span)?;
    Ok(Point::new(x, y))
}

fn read_relative_point(span: &mut &str, origin: Point) -> Result<Point, FormatError> {
    let x = read_double(span)?;
    *span = read_separator(span);
    let y = read_double(span)?;
    Ok(Point::new(origin.x + x, origin.y + y))
}

fn read_command(span: &mut &str) -> Result<Option<(Command, bool)>, FormatError> {
    *span = skip_whitespace(span);
    let mut chars = span.chars();
    let Some(c) = chars.next() else {
        return Ok(None);
    };

    let Some(command) = c.to_uppercase().next().and_then(command_from_char) else {
        return Err(FormatError::from_string(format!("Unexpected path command '{c}'.")));
    };

    let relative = c.is_lowercase();
    *span = chars.as_str();
    Ok(Some((command, relative)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{ArcSegment, LineSegment, PathFigure, PathGeometry};
    use crate::platform::PathGeometryContext;
    use crate::Ref;

    fn parse(path_data: &str) -> Result<Ref<PathGeometry>, FormatError> {
        let path_geometry = PathGeometry::new();
        let mut context = PathGeometryContext::new(path_geometry.clone());
        let mut parser = PathMarkupParser::new(&mut context);
        let result = parser.parse(path_data);
        parser.dispose();
        context.dispose();
        result.map(|_| path_geometry)
    }

    fn figure(geometry: &PathGeometry, index: usize) -> Ref<PathFigure> {
        geometry.figures().unwrap().get(index)
    }

    fn line_point(figure: &PathFigure, index: usize) -> Point {
        figure.segments().unwrap().get(index).cast::<LineSegment>().expect("line segment").point()
    }

    #[test]
    fn parses_move() {
        let path_geometry = parse("M10 10").unwrap();
        assert_eq!(Point::new(10.0, 10.0), figure(&path_geometry, 0).start_point());
    }

    #[test]
    fn parses_line() {
        let path_geometry = parse("M0 0L10 10").unwrap();
        assert_eq!(Point::new(10.0, 10.0), line_point(&figure(&path_geometry, 0), 0));
    }

    #[test]
    fn parses_close() {
        let path_geometry = parse("M0 0L10 10z").unwrap();
        assert!(figure(&path_geometry, 0).is_closed());
    }

    #[test]
    fn parses_fill_mode_before_move() {
        let path_geometry = parse("F 1M0,0").unwrap();
        assert_eq!(FillRule::NonZero, path_geometry.fill_rule());
    }

    #[test]
    fn parses_implicit_line_command_after_move() {
        for path_data in ["M0 0 10 10 20 20", "M0,0 10,10 20,20", "M0,0,10,10,20,20"] {
            let path_geometry = parse(path_data).unwrap();
            let figure = figure(&path_geometry, 0);
            assert_eq!(Point::new(10.0, 10.0), line_point(&figure, 0));
            assert_eq!(Point::new(20.0, 20.0), line_point(&figure, 1));
        }
    }

    #[test]
    fn parses_implicit_line_command_after_relative_move() {
        for path_data in ["m0 0 10 10 20 20", "m0,0 10,10 20,20", "m0,0,10,10,20,20"] {
            let path_geometry = parse(path_data).unwrap();
            let figure = figure(&path_geometry, 0);
            assert_eq!(Point::new(10.0, 10.0), line_point(&figure, 0));
            assert_eq!(Point::new(30.0, 30.0), line_point(&figure, 1));
        }
    }

    #[test]
    fn parses_scientific_notation_double() {
        let path_geometry = parse("M -1.01725E-005 -1.01725e-005").unwrap();
        assert_eq!(Point::new(-1.01725E-005, -1.01725E-005), figure(&path_geometry, 0).start_point());
    }

    #[test]
    fn should_parse() {
        for path_data in [
            "M5.5.5 5.5.5 5.5.5",
            concat!(
                "F1M9.0771,11C9.1161,10.701,9.1801,10.352,9.3031,10L9.0001,10 9.0001,6.166 3.0001,9.767 3.0001,10 ",
                "9.99999999997669E-05,10 9.99999999997669E-05,0 3.0001,0 3.0001,0.234 9.0001,3.834 9.0001,0 ",
                "12.0001,0 12.0001,8.062C12.1861,8.043 12.3821,8.031 12.5941,8.031 15.3481,8.031 15.7961,9.826 ",
                "15.9201,11L16.0001,16 9.0001,16 9.0001,12.562 9.0001,11z"
            ),
            "         M0 0",
            "F1 M24,14 A2,2,0,1,1,20,14 A2,2,0,1,1,24,14 z",
            "M0 0L10 10z",
            "M50 50 L100 100 L150 50",
            "M50 50L100 100L150 50",
            "M50,50 L100,100 L150,50",
            "M50 50 L-10 -10 L10 50",
            "M50 50L-10-10L10 50",
            "M50 50 L100 100 L150 50zM50 50 L70 70 L120 50z",
            "M 50 50 L 100 100 L 150 50",
            "M50 50 L100 100 L150 50 H200 V100Z",
            "M 80 200 A 100 50 45 1 0 100 50",
            concat!(
                "F1 M 16.6309 18.6563C 17.1309 8.15625 29.8809 14.1563 29.8809 14.1563C 30.8809 11.1563 34.1308 11.4063",
                " 34.1308 11.4063C 33.5 12 34.6309 13.1563 34.6309 13.1563C 32.1309 13.1562 31.1309 14.9062 31.1309 14.9",
                "062C 41.1309 23.9062 32.6309 27.9063 32.6309 27.9062C 24.6309 24.9063 21.1309 22.1562 16.6309 18.6563 Z",
                " M 16.6309 19.9063C 21.6309 24.1563 25.1309 26.1562 31.6309 28.6562C 31.6309 28.6562 26.3809 39.1562 18",
                ".3809 36.1563C 18.3809 36.1563 18 38 16.3809 36.9063C 15 36 16.3809 34.9063 16.3809 34.9063C 16.3809 34",
                ".9063 10.1309 30.9062 16.6309 19.9063 Z "
            ),
            concat!(
                "F1M16,12C16,14.209 14.209,16 12,16 9.791,16 8,14.209 8,12 8,11.817 8.03,11.644 8.054,11.467L6.585,10 4,10 ",
                "4,6.414 2.5,7.914 0,5.414 0,3.586 3.586,0 4.414,0 7.414,3 7.586,3 9,1.586 11.914,4.5 10.414,6 ",
                "12.461,8.046C14.45,8.278,16,9.949,16,12"
            ),
        ] {
            assert!(parse(path_data).is_ok(), "{path_data}");
        }
    }

    #[derive(Default)]
    struct CountingContext {
        end_figure_calls: usize,
    }

    impl IGeometryContext for CountingContext {
        fn arc_to(&mut self, _: Point, _: Size, _: f64, _: bool, _: SweepDirection, _: bool) {}
        fn begin_figure(&mut self, _: Point, _: bool) {}
        fn cubic_bezier_to(&mut self, _: Point, _: Point, _: Point, _: bool) {}
        fn quadratic_bezier_to(&mut self, _: Point, _: Point, _: bool) {}
        fn line_to(&mut self, _: Point, _: bool) {}
        fn end_figure(&mut self, _: bool) {
            self.end_figure_calls += 1;
        }
        fn set_fill_rule(&mut self, _: FillRule) {}
        fn dispose(&mut self) {}
    }

    #[test]
    fn should_always_end_figure() {
        for path_data in
            ["M0 0L10 10", "M0 0L10 10z", "M0 0L10 10 \n ", "M0 0L10 10z \n ", "M0 0L10 10 ", "M0 0L10 10z "]
        {
            let mut context = CountingContext::default();
            PathMarkupParser::new(&mut context).parse(path_data).unwrap();
            assert!(context.end_figure_calls >= 1, "{path_data:?}");
        }
    }

    #[test]
    fn parsed_geometry_to_string_should_produce_valid_value() {
        for path_data in [
            "M 5.5, 5 L 5.5, 5 L 5.5, 5",
            concat!(
                "F1 M 9.0771, 11 C 9.1161, 10.701 9.1801, 10.352 9.3031, 10 L 9.0001, 10 L 9.0001, 6.166 L 3.0001, 9.767 L 3.0001, 10 ",
                "L 9.99999999997669E-05, 10 L 9.99999999997669E-05, 0 L 3.0001, 0 L 3.0001, 0.234 L 9.0001, 3.834 L 9.0001, 0 ",
                "L 12.0001, 0 L 12.0001, 8.062 C 12.1861, 8.043 12.3821, 8.031 12.5941, 8.031 C 15.3481, 8.031 15.7961, 9.826 ",
                "15.9201, 11 L 16.0001, 16 L 9.0001, 16 L 9.0001, 12.562 L 9.0001, 11Z"
            ),
            "F1 M 24, 14 A 2, 2 0 1 1 20, 14 A 2, 2 0 1 1 24, 14Z",
            "M 0, 0 L 10, 10Z",
            "M 50, 50 L 100, 100 L 150, 50",
            "M 50, 50 L -10, -10 L 10, 50",
            "M 50, 50 L 100, 100 L 150, 50Z M 50, 50 L 70, 70 L 120, 50Z",
            "M 80, 200 A 100, 50 45 1 0 100, 50",
            concat!(
                "F1 M 16, 12 C 16, 14.209 14.209, 16 12, 16 C 9.791, 16 8, 14.209 8, 12 C 8, 11.817 8.03, 11.644 8.054, 11.467 L 6.585, 10 ",
                "L 4, 10 L 4, 6.414 L 2.5, 7.914 L 0, 5.414 L 0, 3.586 L 3.586, 0 L 4.414, 0 L 7.414, 3 L 7.586, 3 L 9, 1.586 L ",
                "11.914, 4.5 L 10.414, 6 L 12.461, 8.046 C 14.45, 8.278 16, 9.949 16, 12"
            ),
        ] {
            let target = PathGeometry::parse(path_data).unwrap();
            assert_eq!(path_data, target.to_string());
        }
    }

    #[test]
    fn parsed_geometry_to_string_should_format_value() {
        for (path_data, formatted_path_data) in [
            ("M5.5.5 5.5.5 5.5.5", "M 5.5, 0.5 L 5.5, 0.5 L 5.5, 0.5"),
            (
                "F1 M24,14 A2,2,0,1,1,20,14 A2,2,0,1,1,24,14 z",
                "F1 M 24, 14 A 2, 2 0 1 1 20, 14 A 2, 2 0 1 1 24, 14Z",
            ),
            (
                concat!(
                    "F1M16,12C16,14.209 14.209,16 12,16 9.791,16 8,14.209 8,12 8,11.817 8.03,11.644 8.054,11.467L6.585,10 4,10 ",
                    "4,6.414 2.5,7.914 0,5.414 0,3.586 3.586,0 4.414,0 7.414,3 7.586,3 9,1.586 11.914,4.5 10.414,6 ",
                    "12.461,8.046C14.45,8.278,16,9.949,16,12"
                ),
                concat!(
                    "F1 M 16, 12 C 16, 14.209 14.209, 16 12, 16 C 9.791, 16 8, 14.209 8, 12 C 8, 11.817 8.03, 11.644 8.054, 11.467 L 6.585, 10 ",
                    "L 4, 10 L 4, 6.414 L 2.5, 7.914 L 0, 5.414 L 0, 3.586 L 3.586, 0 L 4.414, 0 L 7.414, 3 L 7.586, 3 L 9, 1.586 L ",
                    "11.914, 4.5 L 10.414, 6 L 12.461, 8.046 C 14.45, 8.278 16, 9.949 16, 12"
                ),
            ),
        ] {
            let target = PathGeometry::parse(path_data).unwrap();
            assert_eq!(formatted_path_data, target.to_string());
        }
    }

    #[test]
    fn returns_error_on_none_defined_command() {
        for path_data in ["0 0", "j"] {
            assert!(parse(path_data).is_err(), "{path_data}");
        }
    }

    #[test]
    fn returns_error_on_malformed_arguments() {
        for path_data in ["M", "M10", "M10 a", "F2M0 0", "F", "M0 0 A1 1 0 2 0 1 1", "M0 0 A1 1 0 1", "M1e", "M1e+", "L.", "M0 0 H", "é"]
        {
            assert!(parse(path_data).is_err(), "{path_data}");
        }
    }

    #[test]
    fn close_figure_should_move_current_point_to_create_figure_point() {
        let path_geometry = parse("M10,10L100,100Z m10,10").unwrap();
        assert_eq!(2, path_geometry.figures().unwrap().len());

        let first = figure(&path_geometry, 0);
        assert_eq!(Point::new(10.0, 10.0), first.start_point());
        assert!(first.is_closed());
        assert_eq!(Point::new(100.0, 100.0), line_point(&first, 0));

        let second = figure(&path_geometry, 1);
        assert_eq!(Point::new(20.0, 20.0), second.start_point());
    }

    #[test]
    fn should_parse_flags_without_separator() {
        let path_geometry = parse("a.898.898 0 01.27.188").unwrap();
        let segments = figure(&path_geometry, 0).segments().unwrap();
        assert_eq!(1, segments.len());
        assert!(segments.get(0).is::<ArcSegment>());
    }

    #[test]
    fn should_handle_start_point_after_empty_figure() {
        let path_geometry = parse("M50,50z l -5,-5").unwrap();
        assert_eq!(2, path_geometry.figures().unwrap().len());
        assert_eq!(Point::new(50.0, 50.0), figure(&path_geometry, 0).start_point());
        assert_eq!(Point::new(50.0, 50.0), figure(&path_geometry, 1).start_point());
    }

    #[test]
    fn smooth_curves_mirror_the_previous_control_point() {
        let path_geometry = parse("M0,0 C0,10 10,10 10,0 S20,-10 20,0 Q25,5 30,0 T40,0").unwrap();
        let figure = figure(&path_geometry, 0);
        let segments = figure.segments().unwrap();
        assert_eq!(4, segments.len());
        let smooth = segments.get(1).cast::<crate::media::BezierSegment>().unwrap();
        assert_eq!(Point::new(10.0, -10.0), smooth.point1());
        assert_eq!(Point::new(20.0, -10.0), smooth.point2());
        assert_eq!(Point::new(20.0, 0.0), smooth.point3());
        let smooth_quadratic = segments.get(3).cast::<crate::media::QuadraticBezierSegment>().unwrap();
        assert_eq!(Point::new(35.0, -5.0), smooth_quadratic.point1());
        assert_eq!(Point::new(40.0, 0.0), smooth_quadratic.point2());
    }
}
