//! `path!`: path data checked when the crate is compiled and written as a static table.
//!
//! This reads the grammar of `PathMarkupParser` of `ferroui-base` and makes the same calls on its
//! geometry context, recorded as commands. The macro crate cannot depend on `ferroui-base` (it
//! would build the framework for the host as well), so the grammar is written here again, and
//! the tests of `path!` in `ferroui-view` compare the two on the data of the parser's tests.

use proc_macro2::{Literal, Punct, Spacing, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};

/// One call on the geometry context, as `PathCommand` of `ferroui-base`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Command {
    SetFillRule { non_zero: bool },
    BeginFigure(Point),
    LineTo(Point),
    QuadraticBezierTo(Point, Point),
    CubicBezierTo(Point, Point, Point),
    ArcTo { end: Point, size: Point, rotation: f64, is_large_arc: bool, clockwise: bool },
    EndFigure { closed: bool },
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Point {
    x: f64,
    y: f64,
}

impl std::ops::Add for Point {
    type Output = Point;
    fn add(self, other: Point) -> Point {
        Point { x: self.x + other.x, y: self.y + other.y }
    }
}

/// What is wrong with path data, and how far into the text.
#[derive(Debug, PartialEq)]
pub(crate) struct PathError {
    pub(crate) offset: usize,
    pub(crate) message: String,
}

struct Reader<'a> {
    text: &'a str,
    rest: &'a str,
}

impl<'a> Reader<'a> {
    fn offset(&self) -> usize {
        self.text.len() - self.rest.len()
    }

    fn error<T>(&self, message: impl Into<String>) -> Result<T, PathError> {
        Err(PathError { offset: self.offset(), message: message.into() })
    }

    fn skip_whitespace(&mut self) {
        self.rest = self.rest.trim_start_matches(char::is_whitespace);
    }

    fn separator(&mut self) {
        self.skip_whitespace();
        self.rest = self.rest.strip_prefix(',').unwrap_or(self.rest);
    }

    fn peek_argument(&mut self) -> bool {
        let rest = self.rest.trim_start_matches(char::is_whitespace);
        matches!(rest.chars().next(), Some(c) if c == ',' || c == '-' || c == '.' || c.is_ascii_digit())
    }

    /// One numeric argument: `-`, digits, `.` and digits, and an exponent with a sign.
    fn argument(&mut self) -> Option<&'a str> {
        self.skip_whitespace();
        let bytes = self.rest.as_bytes();
        if bytes.is_empty() {
            return None;
        }
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
        if i < bytes.len() && (bytes[i] == b'E' || bytes[i] == b'e') {
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
        if !valid {
            return None;
        }
        let (argument, rest) = self.rest.split_at(i);
        self.rest = rest;
        Some(argument)
    }

    fn double(&mut self) -> Result<f64, PathError> {
        let Some(argument) = self.argument() else {
            return self.error("expected a number");
        };
        match argument.parse::<f64>() {
            Ok(value) if value.is_finite() => Ok(value),
            _ => self.error(format!("`{argument}` is not a finite number")),
        }
    }

    fn flag(&mut self) -> Result<bool, PathError> {
        self.skip_whitespace();
        let mut chars = self.rest.chars();
        let value = match chars.next() {
            Some('0') => false,
            Some('1') => true,
            _ => return self.error("expected an arc flag, `0` or `1`"),
        };
        self.rest = chars.as_str();
        Ok(value)
    }

    fn point(&mut self) -> Result<Point, PathError> {
        let x = self.double()?;
        self.separator();
        let y = self.double()?;
        Ok(Point { x, y })
    }

    fn point_from(&mut self, relative: bool, origin: Point) -> Result<Point, PathError> {
        let point = self.point()?;
        Ok(if relative { origin + point } else { point })
    }
}

#[derive(Default)]
struct Figures {
    commands: Vec<Command>,
    current: Point,
    begin: Option<Point>,
    previous_control: Option<Point>,
    is_open: bool,
}

impl Figures {
    fn create_figure(&mut self) {
        if self.is_open {
            self.commands.push(Command::EndFigure { closed: false });
        }
        self.commands.push(Command::BeginFigure(self.current));
        self.begin = Some(self.current);
        self.is_open = true;
    }

    fn ensure_figure(&mut self) {
        if !self.is_open {
            self.create_figure();
        }
    }

    fn close(&mut self) {
        if self.is_open {
            self.commands.push(Command::EndFigure { closed: true });
            if let Some(begin) = self.begin.take() {
                self.current = begin;
            }
        }
        self.previous_control = None;
        self.is_open = false;
    }

    fn line(&mut self, reader: &mut Reader<'_>, relative: bool) -> Result<(), PathError> {
        let next = reader.point_from(relative, self.current)?;
        self.line_to(next);
        Ok(())
    }

    fn line_to(&mut self, next: Point) {
        self.ensure_figure();
        self.commands.push(Command::LineTo(next));
        self.current = next;
    }

    fn mirrored_control(&mut self) {
        if let Some(control) = self.previous_control {
            let center = self.current;
            self.previous_control =
                Some(Point { x: center.x + -(control.x - center.x), y: center.y + -(control.y - center.y) });
        }
    }

    fn command(&mut self, reader: &mut Reader<'_>, command: char, relative: bool) -> Result<(), PathError> {
        match command {
            'F' => {
                let at = reader.offset();
                let rule = match reader.argument() {
                    Some("0") => false,
                    Some("1") => true,
                    _ => return Err(PathError { offset: at, message: "the fill rule is `F0` or `F1`".to_string() }),
                };
                self.commands.push(Command::SetFillRule { non_zero: rule });
            }
            'M' => {
                self.current = reader.point_from(relative, self.current)?;
                self.create_figure();
                while reader.peek_argument() {
                    reader.separator();
                    self.line(reader, relative)?;
                }
            }
            'L' => self.line(reader, relative)?,
            'H' => {
                let value = reader.double()?;
                let x = if relative { self.current.x + value } else { value };
                self.line_to(Point { x, y: self.current.y });
            }
            'V' => {
                let value = reader.double()?;
                let y = if relative { self.current.y + value } else { value };
                self.line_to(Point { x: self.current.x, y });
            }
            'C' => {
                let first = reader.point_from(relative, self.current)?;
                reader.separator();
                let second = reader.point_from(relative, self.current)?;
                self.previous_control = Some(second);
                reader.separator();
                let end = reader.point_from(relative, self.current)?;
                self.ensure_figure();
                self.commands.push(Command::CubicBezierTo(first, second, end));
                self.current = end;
            }
            'Q' => {
                let control = reader.point_from(relative, self.current)?;
                self.previous_control = Some(control);
                reader.separator();
                let end = reader.point_from(relative, self.current)?;
                self.ensure_figure();
                self.commands.push(Command::QuadraticBezierTo(control, end));
                self.current = end;
            }
            'S' => {
                let second = reader.point_from(relative, self.current)?;
                reader.separator();
                let end = reader.point_from(relative, self.current)?;
                self.mirrored_control();
                self.ensure_figure();
                let first = self.previous_control.unwrap_or(self.current);
                self.commands.push(Command::CubicBezierTo(first, second, end));
                self.previous_control = Some(second);
                self.current = end;
            }
            'T' => {
                let end = reader.point_from(relative, self.current)?;
                self.mirrored_control();
                self.ensure_figure();
                let control = self.previous_control.unwrap_or(self.current);
                self.commands.push(Command::QuadraticBezierTo(control, end));
                self.current = end;
            }
            'A' => {
                let width = reader.double()?;
                reader.separator();
                let height = reader.double()?;
                reader.separator();
                let rotation = reader.double()?;
                reader.separator();
                let is_large_arc = reader.flag()?;
                reader.separator();
                let clockwise = reader.flag()?;
                reader.separator();
                let end = reader.point_from(relative, self.current)?;
                self.ensure_figure();
                self.commands.push(Command::ArcTo {
                    end,
                    size: Point { x: width, y: height },
                    rotation,
                    is_large_arc,
                    clockwise,
                });
                self.current = end;
                self.previous_control = None;
            }
            'Z' => self.close(),
            _ => unreachable!("the command was checked"),
        }
        Ok(())
    }
}

/// The commands of `text`, or what is wrong with it.
pub(crate) fn parse(text: &str) -> Result<Vec<Command>, PathError> {
    let mut reader = Reader { text, rest: text };
    let mut figures = Figures::default();
    loop {
        reader.skip_whitespace();
        let mut chars = reader.rest.chars();
        let Some(c) = chars.next() else { break };
        let command = c.to_uppercase().next().unwrap_or(c);
        if !"FMLHVQTCSAZ".contains(command) {
            return reader.error(format!("`{c}` is not a path command; use one of M L H V C S Q T A Z, or F0 / F1"));
        }
        let relative = c.is_lowercase();
        reader.rest = chars.as_str();
        let mut initial = true;
        loop {
            if !initial {
                reader.separator();
            }
            figures.command(&mut reader, command, relative)?;
            initial = false;
            if !reader.peek_argument() {
                break;
            }
        }
    }
    if figures.is_open {
        figures.commands.push(Command::EndFigure { closed: false });
    }
    Ok(figures.commands)
}

/// A number as tokens: a literal of its exact value, with its sign apart.
struct Number(f64);

impl ToTokens for Number {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if self.0.is_sign_negative() {
            tokens.append(Punct::new('-', Spacing::Alone));
        }
        tokens.append(Literal::f64_suffixed(self.0.abs()));
    }
}

fn point(point: Point) -> TokenStream {
    let (x, y) = (Number(point.x), Number(point.y));
    quote! { ::ferroui_view::__path::Point::new(#x, #y) }
}

fn command(command: &Command) -> TokenStream {
    let path = quote! { ::ferroui_view::__path::PathCommand };
    match *command {
        Command::SetFillRule { non_zero } => {
            let rule = if non_zero { quote!(NonZero) } else { quote!(EvenOdd) };
            quote! { #path::SetFillRule(::ferroui_view::__path::FillRule::#rule) }
        }
        Command::BeginFigure(start) => {
            let start = self::point(start);
            quote! { #path::BeginFigure(#start, true) }
        }
        Command::LineTo(end) => {
            let end = self::point(end);
            quote! { #path::LineTo(#end, true) }
        }
        Command::QuadraticBezierTo(control, end) => {
            let (control, end) = (self::point(control), self::point(end));
            quote! { #path::QuadraticBezierTo(#control, #end, true) }
        }
        Command::CubicBezierTo(first, second, end) => {
            let (first, second, end) = (self::point(first), self::point(second), self::point(end));
            quote! { #path::CubicBezierTo(#first, #second, #end, true) }
        }
        Command::ArcTo { end, size, rotation, is_large_arc, clockwise } => {
            let end = self::point(end);
            let (width, height, rotation) = (Number(size.x), Number(size.y), Number(rotation));
            let sweep = if clockwise { quote!(Clockwise) } else { quote!(CounterClockwise) };
            quote! {
                #path::ArcTo(
                    #end,
                    ::ferroui_view::__path::Size::new(#width, #height),
                    #rotation,
                    #is_large_arc,
                    ::ferroui_view::__path::SweepDirection::#sweep,
                    true,
                )
            }
        }
        Command::EndFigure { closed } => quote! { #path::EndFigure(#closed) },
    }
}

/// The expansion of `path!("...")`.
pub(crate) fn expand(literal: &syn::LitStr) -> syn::Result<TokenStream> {
    let text = literal.value();
    let commands = parse(&text).map_err(|error| {
        syn::Error::new(
            literal.span(),
            format!(
                "invalid path data at byte {}: {}. A path reads like \"M8 3v5H3v2h5v5h2V10h5V8h-5V3z\"",
                error.offset, error.message
            ),
        )
    })?;
    let count = commands.len();
    let commands = commands.iter().map(command);
    Ok(quote! {{
        static COMMANDS: [::ferroui_view::__path::PathCommand; #count] = [#(#commands),*];
        ::ferroui_view::PathData::from_static(&COMMANDS)
    }})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_data_is_refused_with_its_position() {
        assert_eq!(parse("M0 0 X1").unwrap_err().offset, 5);
        assert!(parse("M0").is_err());
        assert!(parse("F2 M0 0").is_err());
        assert!(parse("M0 0 A1 1 0 2 0 1 1").is_err());
        assert!(parse("M1e999 0").is_err());
    }

    #[test]
    fn the_commands_of_a_small_path() {
        let commands = parse("M1 2 l3 4z").unwrap();
        assert_eq!(
            commands,
            [
                Command::BeginFigure(Point { x: 1.0, y: 2.0 }),
                Command::LineTo(Point { x: 4.0, y: 6.0 }),
                Command::EndFigure { closed: true },
            ]
        );
    }
}
