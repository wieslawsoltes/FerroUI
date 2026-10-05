use crate::media::string_splitter::{split_respecting_brackets, StringSplitOptions};
use crate::media::Color;
use crate::utilities::span_helpers::{parse_double, write_double};
use crate::utilities::FormatError;
use crate::{Rect, Vector};
use std::fmt;
use std::str::FromStr;

const SEPARATORS: [char; 2] = [' ', '\t'];
const OPENING_PARENTHESIS: char = '(';
const CLOSING_PARENTHESIS: char = ')';

/// Represents a box shadow which can be attached to an element or control.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoxShadow {
    /// The horizontal offset (distance) of the shadow.
    pub offset_x: f64,
    /// The vertical offset (distance) of the shadow.
    pub offset_y: f64,
    /// The blur radius. A higher value results in a more blurred shadow.
    pub blur: f64,
    /// The spread radius. Positive values make the shadow grow, negative
    /// values make it shrink.
    pub spread: f64,
    /// The color of the shadow.
    pub color: Color,
    /// Whether the shadow is an inner shadow (inset) rather than an outer
    /// shadow (outset).
    pub is_inset: bool,
}

impl BoxShadow {
    /// Parses a `BoxShadow` string: `[inset] offset-x offset-y [blur [spread]] color`,
    /// or `none`.
    pub fn parse(s: &str) -> Result<BoxShadow, FormatError> {
        if s.is_empty() {
            return Err(FormatError::default());
        }

        let p = split_respecting_brackets(
            s,
            &SEPARATORS,
            OPENING_PARENTHESIS,
            CLOSING_PARENTHESIS,
            StringSplitOptions::REMOVE_EMPTY_ENTRIES,
        )?;
        if p.len() == 1 && p[0] == "none" {
            return Ok(BoxShadow::default());
        }

        if p.len() < 3 || p.len() > 6 {
            return Err(FormatError::default());
        }

        let mut inset = false;
        let mut tokens = p.iter().copied();
        let mut read_string = || tokens.next().ok_or_else(FormatError::default);

        let mut first_token = read_string()?;
        if first_token == "inset" {
            inset = true;
            first_token = read_string()?;
        }

        let parse = |token: &str| parse_double(token).ok_or_else(|| FormatError::invalid_input(token));

        let offset_x = parse(first_token)?;
        let offset_y = parse(read_string()?)?;
        let mut blur = 0.0;
        let mut spread = 0.0;

        let token3 = read_string().ok();
        let token4 = read_string().ok();
        let token5 = read_string().ok();

        if token4.is_some() {
            blur = parse(token3.ok_or_else(FormatError::default)?)?;
        }
        if token5.is_some() {
            spread = parse(token4.ok_or_else(FormatError::default)?)?;
        }

        let color = Color::parse(token5.or(token4).or(token3).ok_or_else(FormatError::default)?)?;
        Ok(BoxShadow { is_inset: inset, offset_x, offset_y, blur, spread, color })
    }

    /// Transforms the specified bounding rectangle to account for the shadow's
    /// offset, spread, and blur.
    pub fn transform_bounds(&self, rect: Rect) -> Rect {
        if self.is_inset {
            rect
        } else {
            rect.translate(Vector::new(self.offset_x, self.offset_y)).inflate(self.spread + self.blur)
        }
    }
}

impl FromStr for BoxShadow {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        BoxShadow::parse(s)
    }
}

impl fmt::Display for BoxShadow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == BoxShadow::default() {
            return f.write_str("none");
        }

        if self.is_inset {
            f.write_str("inset ")?;
        }

        write_double(f, self.offset_x)?;
        f.write_str(" ")?;
        write_double(f, self.offset_y)?;
        f.write_str(" ")?;

        if self.blur != 0.0 || self.spread != 0.0 {
            write_double(f, self.blur)?;
            f.write_str(" ")?;
        }

        if self.spread != 0.0 {
            write_double(f, self.spread)?;
            f.write_str(" ")?;
        }

        fmt::Display::fmt(&self.color, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{BoxShadows, Colors};

    #[test]
    fn box_shadow_should_parse() {
        for extra_spaces in [false, true] {
            for inset in [false, true] {
                for (color, expected_color) in
                    [("red", Colors::RED), ("#FF122403", Color::from_uint32(0xFF122403))]
                {
                    for component_count in 2..5 {
                        let mut s = format!("{}10 20", if inset { "inset " } else { "" });
                        if component_count > 2 {
                            s += " 30";
                        }
                        if component_count > 3 {
                            s += " 40";
                        }
                        s += " ";
                        s += color;
                        if extra_spaces {
                            s = format!(" {}   ", s.replace(' ', "  "));
                        }

                        let parsed = BoxShadow::parse(&s).unwrap();
                        assert_eq!(inset, parsed.is_inset);
                        assert_eq!(10.0, parsed.offset_x);
                        assert_eq!(20.0, parsed.offset_y);
                        assert_eq!(if component_count > 2 { 30.0 } else { 0.0 }, parsed.blur);
                        assert_eq!(if component_count > 3 { 40.0 } else { 0.0 }, parsed.spread);
                        assert_eq!(expected_color, parsed.color);
                    }
                }
            }
        }
    }

    fn assert_to_string(source: BoxShadows, expected: &str) {
        assert!(expected.eq_ignore_ascii_case(&source.to_string()), "{source} != {expected}");
    }

    #[test]
    fn box_shadows_should_to_string() {
        assert_to_string(
            BoxShadows::new(BoxShadow { offset_x: -15.0, offset_y: 20.0, spread: 5.0, color: Colors::RED, ..Default::default() }),
            "-15 20 0 5 red",
        );
        assert_to_string(
            BoxShadows::new(BoxShadow {
                is_inset: true,
                offset_x: -15.0,
                offset_y: 20.0,
                spread: 5.0,
                color: Colors::RED,
                ..Default::default()
            }),
            "inset -15 20 0 5 red",
        );
        assert_to_string(
            BoxShadows::new(BoxShadow { offset_x: -15.0, offset_y: 20.0, blur: 5.0, color: Colors::RED, ..Default::default() }),
            "-15 20 5 red",
        );
        assert_to_string(
            BoxShadows::with_rest(
                BoxShadow {
                    offset_x: -20.0,
                    offset_y: -20.0,
                    blur: 60.0,
                    color: Color::parse("#CCFFFFFF").unwrap(),
                    ..Default::default()
                },
                &[BoxShadow {
                    offset_x: 20.0,
                    offset_y: 20.0,
                    blur: 60.0,
                    color: Color::parse("#33000000").unwrap(),
                    ..Default::default()
                }],
            ),
            "-20 -20 60 #CCFFFFFF, 20 20 60 #33000000",
        );
    }

    #[test]
    fn parse_rejects_invalid_input() {
        assert!(BoxShadow::parse("").is_err());
        assert!(BoxShadow::parse("10 red").is_err());
        assert!(BoxShadow::parse("a b red").is_err());
        assert!(BoxShadow::parse("1 2 3 4 5 6 red").is_err());
        assert!(BoxShadow::parse("1 2 rgba(0,0").is_err());
    }
}
