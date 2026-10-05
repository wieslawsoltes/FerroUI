//! An ARGB color.
//!
//! Color conversion portions of this file are adapted from the WinUI project
//! (https://github.com/microsoft/microsoft-ui-xaml) and the Windows Community
//! Toolkit project (https://github.com/CommunityToolkit/WindowsCommunityToolkit),
//! both MIT licensed.

use std::fmt;
use std::str::FromStr;

use crate::media::{HslColor, HsvColor, KnownColor, KnownColors};
use crate::utilities::span_helpers::{
    try_parse_byte, try_parse_double, try_parse_uint, FixedF2, NumberStyles,
};
use crate::utilities::FormatError;

const BYTE_TO_DOUBLE: f64 = 1.0 / 255.0;

/// An ARGB color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Color {
    /// The Alpha component of the color.
    pub a: u8,
    /// The Red component of the color.
    pub r: u8,
    /// The Green component of the color.
    pub g: u8,
    /// The Blue component of the color.
    pub b: u8,
}

/// Rounds half to even and converts to a byte, saturating at 0 and 255 (NaN becomes 0).
#[inline]
fn round_to_byte(value: f64) -> u8 {
    value.round_ties_even() as u8
}

/// Rounds half to even and converts to an integer (saturating; NaN becomes 0).
#[inline]
fn round_to_int(value: f64) -> i32 {
    value.round_ties_even() as i32
}

/// Length of the string in UTF-16 code units (the unit the format length rules
/// are expressed in).
#[inline]
pub(crate) fn utf16_len(s: &str) -> usize {
    if s.is_ascii() {
        s.len()
    } else {
        s.encode_utf16().count()
    }
}

/// ASCII case-insensitive `starts_with`.
#[inline]
pub(crate) fn starts_with_ignore_ascii_case(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

/// Splits `s` on commas into at most four components. Returns the components and
/// the real component count (which may exceed four).
#[inline]
pub(crate) fn split_components(s: &str) -> ([&str; 4], usize) {
    let mut components = [""; 4];
    let mut count = 0;
    for part in s.split(',') {
        if count < 4 {
            components[count] = part;
        }
        count += 1;
    }
    (components, count)
}

/// Strips a CSS-like function wrapper: `"<name>a(...)"` (when the trimmed string
/// has at least 11 UTF-16 units) or `"<name>(...)"` (at least 10). `with_alpha`
/// and `without_alpha` are the prefixes including the opening parenthesis.
pub(crate) fn strip_css_function<'a>(
    s: &'a str,
    with_alpha: &str,
    without_alpha: &str,
) -> Option<&'a str> {
    let working = s.trim();

    if working.is_empty() || !working.contains(',') {
        return None;
    }

    let length = utf16_len(working);

    if length >= 11 && starts_with_ignore_ascii_case(working, with_alpha) && working.ends_with(')')
    {
        return Some(&working[with_alpha.len()..working.len() - 1]);
    }

    if length >= 10
        && starts_with_ignore_ascii_case(working, without_alpha)
        && working.ends_with(')')
    {
        return Some(&working[without_alpha.len()..working.len() - 1]);
    }

    None
}

/// Parses a double with an optional percent sign. Everything after the first `%`
/// is ignored and the value before it is divided by 100.
pub(crate) fn try_parse_double_or_percent(s: &str) -> Option<f64> {
    match s.find('%') {
        Some(percent_index) => {
            try_parse_double(&s[..percent_index], NumberStyles::NUMBER).map(|p| p / 100.0)
        }
        None => try_parse_double(s, NumberStyles::NUMBER),
    }
}

impl Color {
    /// Initializes a new instance of the [`Color`] struct.
    #[inline]
    pub const fn new(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self { a, r, g, b }
    }

    /// Creates a [`Color`] from alpha, red, green and blue components.
    #[inline]
    pub const fn from_argb(a: u8, r: u8, g: u8, b: u8) -> Color {
        Color::new(a, r, g, b)
    }

    /// Creates an opaque [`Color`] from red, green and blue components.
    #[inline]
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Color {
        Color::new(0xff, r, g, b)
    }

    /// Creates a [`Color`] from an integer in `0xAARRGGBB` layout.
    #[inline]
    pub const fn from_uint32(value: u32) -> Color {
        Color::new(
            ((value >> 24) & 0xff) as u8,
            ((value >> 16) & 0xff) as u8,
            ((value >> 8) & 0xff) as u8,
            (value & 0xff) as u8,
        )
    }

    /// Parses a color string: `#rgb`, `#argb`, `#rrggbb`, `#aarrggbb`,
    /// `rgb()`/`rgba()`, `hsl()`/`hsla()`, `hsv()`/`hsva()` or a known color name.
    pub fn parse(s: &str) -> Result<Color, FormatError> {
        match Color::try_parse(s) {
            Some(color) => Ok(color),
            None => Err(FormatError::from_string(format!(
                "Invalid color string: '{s}'."
            ))),
        }
    }

    /// Parses a color string, returning `None` when it is not a valid color.
    pub fn try_parse(s: &str) -> Option<Color> {
        if s.is_empty() {
            return None;
        }

        let bytes = s.as_bytes();

        if bytes[0] == b'#' {
            if let Some(color) = Color::try_parse_hex_format(s) {
                return Some(color);
            }
        }

        // Note: The length checks are also an important optimization.
        // The shortest possible CSS format is "rgb(0,0,0)", Length = 10.
        if bytes.len() >= 3 && utf16_len(s) >= 10 {
            let prefix = [
                bytes[0].to_ascii_lowercase(),
                bytes[1].to_ascii_lowercase(),
                bytes[2].to_ascii_lowercase(),
            ];

            if &prefix == b"rgb" {
                if let Some(color) = Color::try_parse_css_format(s) {
                    return Some(color);
                }
            }

            if &prefix == b"hsl" {
                if let Some(hsl_color) = HslColor::try_parse(s) {
                    return Some(hsl_color.to_rgb());
                }
            }

            if &prefix == b"hsv" {
                if let Some(hsv_color) = HsvColor::try_parse(s) {
                    return Some(hsv_color.to_rgb());
                }
            }
        }

        let known_color = KnownColors::get_known_color(s);

        if known_color != KnownColor::NONE {
            return Some(known_color.to_color());
        }

        None
    }

    /// Parses the given string representing a CSS color value into a new [`Color`].
    fn try_parse_hex_format(s: &str) -> Option<Color> {
        fn try_parse_core(input: &[u8]) -> Option<Color> {
            let mut alpha_component = 0u32;

            if input.len() == 6 {
                alpha_component = 0xff000000;
            } else if input.len() != 8 {
                return None;
            }

            let text = std::str::from_utf8(input).ok()?;
            let parsed = try_parse_uint(text, NumberStyles::HEX_NUMBER)?;

            Some(Color::from_uint32(parsed | alpha_component))
        }

        let input = &s.as_bytes()[1..];

        // Anything outside ASCII can be neither a hex digit nor skippable white space.
        if !input.is_ascii() {
            return None;
        }

        // Handle shorthand cases like #FFF (RGB) or #FFFF (ARGB).
        if input.len() == 3 || input.len() == 4 {
            let extended_length = 2 * input.len();
            let mut extended = [0u8; 8];

            for (i, &c) in input.iter().enumerate() {
                extended[2 * i] = c;
                extended[2 * i + 1] = c;
            }

            return try_parse_core(&extended[..extended_length]);
        }

        try_parse_core(input)
    }

    /// Parses the given string representing a CSS color value (`rgb()`/`rgba()`).
    fn try_parse_css_format(s: &str) -> Option<Color> {
        // Parses a byte value with an optional percentage sign. The percent sign, if
        // it exists, ends the number; anything after it is ignored.
        fn internal_try_parse_byte(s: &str) -> Option<u8> {
            match s.find('%') {
                Some(percent_index) => try_parse_double(&s[..percent_index], NumberStyles::NUMBER)
                    .map(|percentage| round_to_byte((percentage / 100.0) * 255.0)),
                None => try_parse_byte(s, NumberStyles::NUMBER),
            }
        }

        let working = strip_css_function(s, "rgba(", "rgb(")?;
        let (components, count) = split_components(working);

        if count == 3 {
            // RGB
            let red = internal_try_parse_byte(components[0])?;
            let green = internal_try_parse_byte(components[1])?;
            let blue = internal_try_parse_byte(components[2])?;

            Some(Color::new(0xFF, red, green, blue))
        } else if count == 4 {
            // RGBA
            let red = internal_try_parse_byte(components[0])?;
            let green = internal_try_parse_byte(components[1])?;
            let blue = internal_try_parse_byte(components[2])?;
            let alpha = try_parse_double_or_percent(components[3])?;

            Some(Color::new(round_to_byte(alpha * 255.0), red, green, blue))
        } else {
            None
        }
    }

    /// Returns the string representation of the color using a format specifier.
    ///
    /// Supported format specifiers:
    /// - `X`: XAML hex with alpha, `#AARRGGBB`
    /// - `x`: hex without alpha, `#RRGGBB`
    /// - `H`: HTML/CSS hex with alpha, `#RRGGBBAA`
    /// - `R` / `r`: CSS `rgba(r, g, b, a)` / `rgb(r, g, b)`
    /// - `R%` / `r%`: CSS `rgba(r%, g%, b%, a%)` / `rgb(r%, g%, b%)`
    /// - `L`, `l`, `L%`, `l%`: converted to HSL and formatted as such
    /// - `V`, `v`, `V%`, `v%`: converted to HSV and formatted as such
    ///
    /// An empty format is the same as `to_string()`. Any other format is an error.
    pub fn to_string_format(&self, format: &str) -> Result<String, FormatError> {
        if format.is_empty() {
            return Ok(self.to_string());
        }

        let (a, r, g, b) = (self.a, self.r, self.g, self.b);

        Ok(match format {
            "X" => format!("#{a:02X}{r:02X}{g:02X}{b:02X}"),
            "x" => format!("#{r:02X}{g:02X}{b:02X}"),
            "H" => format!("#{r:02X}{g:02X}{b:02X}{a:02X}"),
            "R" => self.format_rgb_css(true),
            "r" => self.format_rgb_css(false),
            "R%" => self.format_rgb_percent_css(true),
            "r%" => self.format_rgb_percent_css(false),
            "L" | "l" | "L%" | "l%" => return self.to_hsl().to_string_format(format),
            "V" | "v" | "V%" | "v%" => return self.to_hsv().to_string_format(format),
            _ => {
                return Err(FormatError::from_string(format!(
                    "Format string '{format}' is not supported."
                )))
            }
        })
    }

    fn format_rgb_css(&self, include_alpha: bool) -> String {
        if include_alpha {
            return format!(
                "rgba({}, {}, {}, {})",
                self.r,
                self.g,
                self.b,
                FixedF2(self.a as f64 * BYTE_TO_DOUBLE)
            );
        }

        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }

    fn format_rgb_percent_css(&self, include_alpha: bool) -> String {
        let r_pct = round_to_int(self.r as f64 * BYTE_TO_DOUBLE * 100.0);
        let g_pct = round_to_int(self.g as f64 * BYTE_TO_DOUBLE * 100.0);
        let b_pct = round_to_int(self.b as f64 * BYTE_TO_DOUBLE * 100.0);

        if include_alpha {
            let a_pct = round_to_int(self.a as f64 * BYTE_TO_DOUBLE * 100.0);
            return format!("rgba({r_pct}%, {g_pct}%, {b_pct}%, {a_pct}%)");
        }

        format!("rgb({r_pct}%, {g_pct}%, {b_pct}%)")
    }

    /// Returns the integer representation of the color in `0xAARRGGBB` layout.
    #[inline]
    pub const fn to_uint32(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Returns the HSL color model equivalent of this RGB color.
    #[inline]
    pub fn to_hsl(&self) -> HslColor {
        Color::rgb_to_hsl(self.r, self.g, self.b, self.a)
    }

    /// Returns the HSV color model equivalent of this RGB color.
    #[inline]
    pub fn to_hsv(&self) -> HsvColor {
        Color::rgb_to_hsv(self.r, self.g, self.b, self.a)
    }

    /// Check if two colors are equal.
    #[inline]
    pub const fn equals(&self, other: Color) -> bool {
        self.a == other.a && self.r == other.r && self.g == other.g && self.b == other.b
    }

    /// Converts the given RGBA color component values to their HSL color equivalent.
    #[inline]
    pub fn rgb_to_hsl(red: u8, green: u8, blue: u8, alpha: u8) -> HslColor {
        // Normalize RGBA components into the 0..1 range
        Color::normalized_rgb_to_hsl(
            BYTE_TO_DOUBLE * red as f64,
            BYTE_TO_DOUBLE * green as f64,
            BYTE_TO_DOUBLE * blue as f64,
            BYTE_TO_DOUBLE * alpha as f64,
        )
    }

    /// Converts the given RGBA color component values to their HSL color equivalent.
    /// Warning: no bounds checks or clamping is done on the input component values;
    /// they are assumed to be within the 0..1 range. The result is not clamped either.
    pub fn normalized_rgb_to_hsl(r: f64, g: f64, b: f64, a: f64) -> HslColor {
        let max = if r >= g {
            if r >= b {
                r
            } else {
                b
            }
        } else if g >= b {
            g
        } else {
            b
        };
        let min = if r <= g {
            if r <= b {
                r
            } else {
                b
            }
        } else if g <= b {
            g
        } else {
            b
        };
        let chroma = max - min;
        let h1;

        if chroma == 0.0 {
            h1 = 0.0;
        } else if max == r {
            // The % operator doesn't do proper modulo on negative
            // numbers, so we'll add 6 before using it
            h1 = (((g - b) / chroma) + 6.0) % 6.0;
        } else if max == g {
            h1 = 2.0 + ((b - r) / chroma);
        } else {
            h1 = 4.0 + ((r - g) / chroma);
        }

        let lightness = 0.5 * (max + min);
        let saturation = if chroma == 0.0 {
            0.0
        } else {
            chroma / (1.0 - ((2.0 * lightness) - 1.0).abs())
        };

        HslColor::new_with_clamp(a, 60.0 * h1, saturation, lightness, false)
    }

    /// Converts the given RGBA color component values to their HSV color equivalent.
    #[inline]
    pub fn rgb_to_hsv(red: u8, green: u8, blue: u8, alpha: u8) -> HsvColor {
        // Normalize RGBA components into the 0..1 range
        Color::normalized_rgb_to_hsv(
            BYTE_TO_DOUBLE * red as f64,
            BYTE_TO_DOUBLE * green as f64,
            BYTE_TO_DOUBLE * blue as f64,
            BYTE_TO_DOUBLE * alpha as f64,
        )
    }

    /// Converts the given RGBA color component values to their HSV color equivalent.
    /// Warning: no bounds checks or clamping is done on the input component values;
    /// they are assumed to be within the 0..1 range. The result is not clamped either.
    pub fn normalized_rgb_to_hsv(r: f64, g: f64, b: f64, a: f64) -> HsvColor {
        let mut hue;
        let saturation;

        let max = if r >= g {
            if r >= b {
                r
            } else {
                b
            }
        } else if g >= b {
            g
        } else {
            b
        };
        let min = if r <= g {
            if r <= b {
                r
            } else {
                b
            }
        } else if g <= b {
            g
        } else {
            b
        };

        // The value, a number between 0 and 1, is the largest of R, G, and B.
        // Conceptually speaking, it represents how much color is present.
        let value = max;

        // The "chroma" of the color is a value directly proportional to the extent to which
        // the color diverges from greyscale.
        let chroma = max - min;

        // If the chroma is zero, then hue is technically undefined - a greyscale color
        // has no hue. For the sake of convenience, hue is set to zero. Since the color
        // is purely gray, saturation is also equal to zero.
        if chroma == 0.0 {
            hue = 0.0;
            saturation = 0.0;
        } else {
            // Hue can be thought of as a cyclical thing, between 0 degrees and 360 degrees.
            // A hue of 0 degrees is red; 120 degrees is green; 240 degrees is blue; and 360 is back to red.
            if r == max {
                // The red channel is the most pronounced channel: somewhere between (-60, 60).
                hue = 60.0 * (g - b) / chroma;
            } else if g == max {
                // Centered in the green third of the color wheel.
                hue = 120.0 + (60.0 * (b - r) / chroma);
            } else {
                // Centered in the blue third of the color wheel.
                hue = 240.0 + (60.0 * (r - g) / chroma);
            }

            // Since we want to work within the range [0, 360), we'll add 360 to any value less than zero -
            // this will bump red values from within -60 to -1 to 300 to 359. The hue is the same at both values.
            if hue < 0.0 {
                hue += 360.0;
            }

            // The saturation is the chroma normalized by the maximum channel (i.e., the value).
            saturation = chroma / value;
        }

        HsvColor::new_with_clamp(a, hue, saturation, value, false)
    }
}

impl FromStr for Color {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        Color::parse(s)
    }
}

/// Writes the known color name when there is one, `#aarrggbb` (lower-case) otherwise.
impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rgb = self.to_uint32();
        match KnownColors::get_known_color_name(rgb) {
            Some(name) => f.write_str(name),
            None => write!(f, "#{rgb:08x}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_argb(result: Color, a: u8, r: u8, g: u8, b: u8) {
        assert_eq!(r, result.r);
        assert_eq!(g, result.g);
        assert_eq!(b, result.b);
        assert_eq!(a, result.a);
    }

    #[test]
    fn parse_parses_rgb_hash_color() {
        let result = Color::parse("#ff8844").unwrap();

        assert_argb(result, 0xff, 0xff, 0x88, 0x44);
    }

    #[test]
    fn try_parse_parses_rgb_hash_color() {
        let result = Color::try_parse("#ff8844");

        assert!(result.is_some());
        assert_argb(result.unwrap(), 0xff, 0xff, 0x88, 0x44);
    }

    #[test]
    fn parse_parses_rgb_hash_shorthand_color() {
        let result = Color::parse("#f84").unwrap();

        assert_argb(result, 0xff, 0xff, 0x88, 0x44);
    }

    #[test]
    fn try_parse_parses_rgb_hash_shorthand_color() {
        let result = Color::try_parse("#f84");

        assert!(result.is_some());
        assert_argb(result.unwrap(), 0xff, 0xff, 0x88, 0x44);
    }

    #[test]
    fn parse_parses_argb_hash_color() {
        let result = Color::parse("#40ff8844").unwrap();

        assert_argb(result, 0x40, 0xff, 0x88, 0x44);
    }

    #[test]
    fn try_parse_parses_argb_hash_color() {
        let result = Color::try_parse("#40ff8844");

        assert!(result.is_some());
        assert_argb(result.unwrap(), 0x40, 0xff, 0x88, 0x44);
    }

    #[test]
    fn parse_parses_argb_hash_shorthand_color() {
        let result = Color::parse("#4f84").unwrap();

        assert_argb(result, 0x44, 0xff, 0x88, 0x44);
    }

    #[test]
    fn try_parse_parses_argb_hash_shorthand_color() {
        let result = Color::try_parse("#4f84");

        assert!(result.is_some());
        assert_argb(result.unwrap(), 0x44, 0xff, 0x88, 0x44);
    }

    #[test]
    fn parse_parses_named_color_lowercase() {
        let result = Color::parse("red").unwrap();

        assert_argb(result, 0xff, 0xff, 0x00, 0x00);
    }

    #[test]
    fn try_parse_parses_named_color_lowercase() {
        let result = Color::try_parse("red");

        assert!(result.is_some());
        assert_argb(result.unwrap(), 0xff, 0xff, 0x00, 0x00);
    }

    #[test]
    fn parse_parses_named_color_uppercase() {
        let result = Color::parse("RED").unwrap();

        assert_argb(result, 0xff, 0xff, 0x00, 0x00);
    }

    #[test]
    fn try_parse_parses_named_color_uppercase() {
        let result = Color::try_parse("RED");

        assert!(result.is_some());
        assert_argb(result.unwrap(), 0xff, 0xff, 0x00, 0x00);
    }

    #[test]
    fn parse_hex_value_doesnt_accept_too_few_chars() {
        assert!(Color::parse("#ff").is_err());
    }

    #[test]
    fn try_parse_hex_value_doesnt_accept_too_few_chars() {
        assert!(Color::try_parse("#ff").is_none());
    }

    #[test]
    fn parse_hex_value_doesnt_accept_too_many_chars() {
        assert!(Color::parse("#ff5555555").is_err());
    }

    #[test]
    fn try_parse_hex_value_doesnt_accept_too_many_chars() {
        assert!(Color::try_parse("#ff5555555").is_none());
    }

    #[test]
    fn parse_hex_value_doesnt_accept_invalid_number() {
        assert!(Color::parse("#ff808g80").is_err());
    }

    #[test]
    fn try_parse_hex_value_doesnt_accept_invalid_number() {
        assert!(Color::try_parse("#ff808g80").is_none());
    }

    #[test]
    fn parse_throws_format_exception_for_invalid_input() {
        assert_eq!(
            Color::parse("").unwrap_err().message(),
            "Invalid color string: ''."
        );
    }

    #[test]
    fn try_parse_returns_false_for_invalid_input() {
        assert!(Color::try_parse("").is_none());
    }

    #[test]
    fn try_parse_hsl_color() {
        let data: [(&str, HslColor); 24] = [
            // HSL
            ("hsl(0, 0, 0)", HslColor::new(1.0, 0.0, 0.0, 0.0)),
            ("hsl(0, 0%, 0%)", HslColor::new(1.0, 0.0, 0.0, 0.0)),
            ("hsl(180, 0.5, 0.5)", HslColor::new(1.0, 180.0, 0.5, 0.5)),
            ("hsl(180, 50%, 50%)", HslColor::new(1.0, 180.0, 0.5, 0.5)),
            ("hsl(360, 1.0, 1.0)", HslColor::new(1.0, 0.0, 1.0, 1.0)), // Wraps Hue to zero
            ("hsl(360, 100%, 100%)", HslColor::new(1.0, 0.0, 1.0, 1.0)), // Wraps Hue to zero
            (
                "hsl(-1000, -1000, -1000)",
                HslColor::new(1.0, 0.0, 0.0, 0.0),
            ), // Clamps to min
            (
                "hsl(-1000, -1000%, -1000%)",
                HslColor::new(1.0, 0.0, 0.0, 0.0),
            ), // Clamps to min
            ("hsl(1000, 1000, 1000)", HslColor::new(1.0, 0.0, 1.0, 1.0)), // Clamps to max (Hue wraps to zero)
            ("hsl(1000, 1000%, 1000%)", HslColor::new(1.0, 0.0, 1.0, 1.0)), // Clamps to max (Hue wraps to zero)
            ("hsl(300, 0.8, 0.2)", HslColor::new(1.0, 300.0, 0.8, 0.2)),
            ("hsl(300, 80%, 20%)", HslColor::new(1.0, 300.0, 0.8, 0.2)),
            // HSLA
            ("hsla(0, 0, 0, 0)", HslColor::new(0.0, 0.0, 0.0, 0.0)),
            ("hsla(0, 0%, 0%, 0%)", HslColor::new(0.0, 0.0, 0.0, 0.0)),
            (
                "hsla(180, 0.5, 0.5, 0.5)",
                HslColor::new(0.5, 180.0, 0.5, 0.5),
            ),
            (
                "hsla(180, 50%, 50%, 50%)",
                HslColor::new(0.5, 180.0, 0.5, 0.5),
            ),
            (
                "hsla(360, 1.0, 1.0, 1.0)",
                HslColor::new(1.0, 0.0, 1.0, 1.0),
            ), // Wraps Hue to zero
            (
                "hsla(360, 100%, 100%, 100%)",
                HslColor::new(1.0, 0.0, 1.0, 1.0),
            ), // Wraps Hue to zero
            (
                "hsla(-1000, -1000, -1000, -1000)",
                HslColor::new(0.0, 0.0, 0.0, 0.0),
            ), // Clamps to min
            (
                "hsla(-1000, -1000%, -1000%, -1000%)",
                HslColor::new(0.0, 0.0, 0.0, 0.0),
            ), // Clamps to min
            (
                "hsla(1000, 1000, 1000, 1000)",
                HslColor::new(1.0, 0.0, 1.0, 1.0),
            ), // Clamps to max (Hue wraps to zero)
            (
                "hsla(1000, 1000%, 1000%, 1000%)",
                HslColor::new(1.0, 0.0, 1.0, 1.0),
            ), // Clamps to max (Hue wraps to zero)
            (
                "hsla(300, 0.9, 0.2, 0.8)",
                HslColor::new(0.8, 300.0, 0.9, 0.2),
            ),
            (
                "hsla(300, 90%, 20%, 0.8)",
                HslColor::new(0.8, 300.0, 0.9, 0.2),
            ),
        ];

        for (text, expected) in data {
            let parsed = HslColor::try_parse(text);
            assert!(parsed.is_some(), "{text}");
            assert!(expected == parsed.unwrap(), "{text}");
        }
    }

    #[test]
    fn try_parse_hsv_color() {
        let data: [(&str, HsvColor); 24] = [
            // HSV
            ("hsv(0, 0, 0)", HsvColor::new(1.0, 0.0, 0.0, 0.0)),
            ("hsv(0, 0%, 0%)", HsvColor::new(1.0, 0.0, 0.0, 0.0)),
            ("hsv(180, 0.5, 0.5)", HsvColor::new(1.0, 180.0, 0.5, 0.5)),
            ("hsv(180, 50%, 50%)", HsvColor::new(1.0, 180.0, 0.5, 0.5)),
            ("hsv(360, 1.0, 1.0)", HsvColor::new(1.0, 0.0, 1.0, 1.0)), // Wraps Hue to zero
            ("hsv(360, 100%, 100%)", HsvColor::new(1.0, 0.0, 1.0, 1.0)), // Wraps Hue to zero
            (
                "hsv(-1000, -1000, -1000)",
                HsvColor::new(1.0, 0.0, 0.0, 0.0),
            ), // Clamps to min
            (
                "hsv(-1000, -1000%, -1000%)",
                HsvColor::new(1.0, 0.0, 0.0, 0.0),
            ), // Clamps to min
            ("hsv(1000, 1000, 1000)", HsvColor::new(1.0, 0.0, 1.0, 1.0)), // Clamps to max (Hue wraps to zero)
            ("hsv(1000, 1000%, 1000%)", HsvColor::new(1.0, 0.0, 1.0, 1.0)), // Clamps to max (Hue wraps to zero)
            ("hsv(300, 0.8, 0.2)", HsvColor::new(1.0, 300.0, 0.8, 0.2)),
            ("hsv(300, 80%, 20%)", HsvColor::new(1.0, 300.0, 0.8, 0.2)),
            // HSVA
            ("hsva(0, 0, 0, 0)", HsvColor::new(0.0, 0.0, 0.0, 0.0)),
            ("hsva(0, 0%, 0%, 0%)", HsvColor::new(0.0, 0.0, 0.0, 0.0)),
            (
                "hsva(180, 0.5, 0.5, 0.5)",
                HsvColor::new(0.5, 180.0, 0.5, 0.5),
            ),
            (
                "hsva(180, 50%, 50%, 50%)",
                HsvColor::new(0.5, 180.0, 0.5, 0.5),
            ),
            (
                "hsva(360, 1.0, 1.0, 1.0)",
                HsvColor::new(1.0, 0.0, 1.0, 1.0),
            ), // Wraps Hue to zero
            (
                "hsva(360, 100%, 100%, 100%)",
                HsvColor::new(1.0, 0.0, 1.0, 1.0),
            ), // Wraps Hue to zero
            (
                "hsva(-1000, -1000, -1000, -1000)",
                HsvColor::new(0.0, 0.0, 0.0, 0.0),
            ), // Clamps to min
            (
                "hsva(-1000, -1000%, -1000%, -1000%)",
                HsvColor::new(0.0, 0.0, 0.0, 0.0),
            ), // Clamps to min
            (
                "hsva(1000, 1000, 1000, 1000)",
                HsvColor::new(1.0, 0.0, 1.0, 1.0),
            ), // Clamps to max (Hue wraps to zero)
            (
                "hsva(1000, 1000%, 1000%, 1000%)",
                HsvColor::new(1.0, 0.0, 1.0, 1.0),
            ), // Clamps to max (Hue wraps to zero)
            (
                "hsva(300, 0.9, 0.2, 0.8)",
                HsvColor::new(0.8, 300.0, 0.9, 0.2),
            ),
            (
                "hsva(300, 90%, 20%, 0.8)",
                HsvColor::new(0.8, 300.0, 0.9, 0.2),
            ),
        ];

        for (text, expected) in data {
            let parsed = HsvColor::try_parse(text);
            assert!(parsed.is_some(), "{text}");
            assert!(expected == parsed.unwrap(), "{text}");
        }
    }

    #[test]
    fn try_parse_all_formats_with_conversion() {
        let data: [(&str, Color); 20] = [
            // RGB
            ("White", Color::new(0xff, 0xff, 0xff, 0xff)),
            ("#123456", Color::new(0xff, 0x12, 0x34, 0x56)),
            ("rgb(100, 30, 45)", Color::new(255, 100, 30, 45)),
            ("rgba(100, 30, 45, 0.9)", Color::new(230, 100, 30, 45)),
            ("rgba(100, 30, 45, 90%)", Color::new(230, 100, 30, 45)),
            ("rgb(255,0,0)", Color::new(255, 255, 0, 0)),
            ("rgb(0,255,0)", Color::new(255, 0, 255, 0)),
            ("rgb(0,0,255)", Color::new(255, 0, 0, 255)),
            ("rgb(100%, 0, 0)", Color::new(255, 255, 0, 0)),
            ("rgb(0, 100%, 0)", Color::new(255, 0, 255, 0)),
            ("rgb(0, 0, 100%)", Color::new(255, 0, 0, 255)),
            ("rgba(0, 0, 100%, 50%)", Color::new(128, 0, 0, 255)),
            ("rgba(50%, 10%, 80%, 50%)", Color::new(128, 128, 26, 204)),
            ("rgba(50%, 10%, 80%, 0.5)", Color::new(128, 128, 26, 204)),
            // HSL
            ("hsl(296, 85%, 12%)", Color::new(255, 53, 5, 57)),
            ("hsla(296, 0.85, 0.12, 0.9)", Color::new(230, 53, 5, 57)),
            ("hsla(296, 85%, 12%, 90%)", Color::new(230, 53, 5, 57)),
            // HSV
            ("hsv(240, 83%, 78%)", Color::new(255, 34, 34, 199)),
            ("hsva(240, 0.83, 0.78, 0.9)", Color::new(230, 34, 34, 199)),
            ("hsva(240, 83%, 78%, 90%)", Color::new(230, 34, 34, 199)),
        ];

        for (text, expected) in data {
            let parsed = Color::try_parse(text);
            assert!(parsed.is_some(), "{text}");
            assert!(expected == parsed.unwrap(), "{text}: {:?}", parsed.unwrap());
        }
    }

    #[test]
    fn hsv_to_from_hsl_conversion() {
        // Note that conversion of values more representative of actual colors is not done due to rounding error
        // It would be necessary to introduce a different equality comparison that accounts for rounding differences in values
        // This is a result of the math in the conversion itself
        // RGB doesn't have this problem because it uses whole numbers
        let data: [(HsvColor, HslColor); 6] = [
            (
                HsvColor::new(1.0, 0.0, 0.0, 0.0),
                HslColor::new(1.0, 0.0, 0.0, 0.0),
            ),
            (
                HsvColor::new(1.0, 359.0, 1.0, 1.0),
                HslColor::new(1.0, 359.0, 1.0, 0.5),
            ),
            (
                HsvColor::new(1.0, 128.0, 0.0, 0.0),
                HslColor::new(1.0, 128.0, 0.0, 0.0),
            ),
            (
                HsvColor::new(1.0, 128.0, 0.0, 1.0),
                HslColor::new(1.0, 128.0, 0.0, 1.0),
            ),
            (
                HsvColor::new(1.0, 128.0, 1.0, 1.0),
                HslColor::new(1.0, 128.0, 1.0, 0.5),
            ),
            (
                HsvColor::new(0.23, 0.5, 1.0, 1.0),
                HslColor::new(0.23, 0.5, 1.0, 0.5),
            ),
        ];

        for (hsv, hsl) in data {
            let converted_hsl = hsv.to_hsl();
            let converted_hsv = hsl.to_hsv();

            assert_eq!(converted_hsv, hsv);
            assert_eq!(converted_hsl, hsl);
        }
    }

    // =====================================================================
    // Unified format specifier tests
    //
    // All three color types (Color, HslColor, HsvColor) support ALL
    // format specifiers. Cross-model formats auto-convert.
    //
    // Convention:
    //   Uppercase = include alpha, a-suffixed prefix (rgba, hsla, hsva)
    //   Lowercase = exclude alpha, plain prefix (rgb, hsl, hsv)
    //   "%" suffix = percent mode
    // =====================================================================

    fn fmt_color(color: Color, format: &str) -> String {
        color.to_string_format(format).unwrap()
    }

    fn fmt_hsl(color: HslColor, format: &str) -> String {
        color.to_string_format(format).unwrap()
    }

    fn fmt_hsv(color: HsvColor, format: &str) -> String {
        color.to_string_format(format).unwrap()
    }

    #[test]
    fn color_to_string_default_returns_known_name() {
        let red = Color::new(0xFF, 0xFF, 0x00, 0x00);
        assert_eq!("Red", red.to_string());
    }

    #[test]
    fn color_to_string_default_returns_hex_for_unknown() {
        let color = Color::new(0x40, 0xFF, 0x88, 0x44);
        assert_eq!("#40ff8844", color.to_string());
    }

    #[test]
    fn color_to_string_empty_format_matches_default() {
        let color = Color::new(0x40, 0xFF, 0x88, 0x44);
        assert_eq!(color.to_string(), fmt_color(color, ""));
    }

    #[test]
    fn hsl_color_to_string_empty_format_matches_default() {
        let color = HslColor::new(0.8, 200.0, 0.6, 0.4);
        assert_eq!(color.to_string(), fmt_hsl(color, ""));
    }

    #[test]
    fn hsv_color_to_string_empty_format_matches_default() {
        let color = HsvColor::new(0.8, 200.0, 0.6, 0.4);
        assert_eq!(color.to_string(), fmt_hsv(color, ""));
    }

    #[test]
    fn color_to_string_x_upper_returns_xaml_hex_with_alpha() {
        let cases: [(u8, u8, u8, u8, &str); 4] = [
            (0xFF, 0xFF, 0x88, 0x44, "#FFFF8844"),
            (0x40, 0xFF, 0x88, 0x44, "#40FF8844"),
            (0xFF, 0x00, 0x00, 0x00, "#FF000000"),
            (0x00, 0x00, 0x00, 0x00, "#00000000"),
        ];
        for (a, r, g, b, expected) in cases {
            assert_eq!(expected, fmt_color(Color::new(a, r, g, b), "X"));
        }
    }

    #[test]
    fn color_to_string_x_lower_returns_hex_without_alpha() {
        let cases: [(u8, u8, u8, u8, &str); 3] = [
            (0xFF, 0xFF, 0x88, 0x44, "#FF8844"),
            (0x40, 0xFF, 0x88, 0x44, "#FF8844"),
            (0x00, 0x00, 0x00, 0x00, "#000000"),
        ];
        for (a, r, g, b, expected) in cases {
            assert_eq!(expected, fmt_color(Color::new(a, r, g, b), "x"));
        }
    }

    #[test]
    fn color_to_string_h_returns_html_hex_with_alpha() {
        let cases: [(u8, u8, u8, u8, &str); 3] = [
            (0xFF, 0xFF, 0x88, 0x44, "#FF8844FF"),
            (0x40, 0xFF, 0x88, 0x44, "#FF884440"),
            (0x00, 0x00, 0x00, 0x00, "#00000000"),
        ];
        for (a, r, g, b, expected) in cases {
            assert_eq!(expected, fmt_color(Color::new(a, r, g, b), "H"));
        }
    }

    #[test]
    fn hsl_color_to_string_x_upper_converts_to_rgb_hex() {
        // Pure red: HSL(0, 1, 0.5) = RGB(255, 0, 0)
        let hsl = HslColor::new(1.0, 0.0, 1.0, 0.5);
        assert_eq!("#FFFF0000", fmt_hsl(hsl, "X"));
    }

    #[test]
    fn hsl_color_to_string_x_lower_converts_to_rgb_hex() {
        let hsl = HslColor::new(1.0, 0.0, 1.0, 0.5);
        assert_eq!("#FF0000", fmt_hsl(hsl, "x"));
    }

    #[test]
    fn hsv_color_to_string_x_upper_converts_to_rgb_hex() {
        // Pure red: HSV(0, 1, 1) = RGB(255, 0, 0)
        let hsv = HsvColor::new(1.0, 0.0, 1.0, 1.0);
        assert_eq!("#FFFF0000", fmt_hsv(hsv, "X"));
    }

    #[test]
    fn hsv_color_to_string_x_lower_converts_to_rgb_hex() {
        let hsv = HsvColor::new(1.0, 0.0, 1.0, 1.0);
        assert_eq!("#FF0000", fmt_hsv(hsv, "x"));
    }

    #[test]
    fn color_to_string_r_upper_returns_rgba_with_alpha() {
        let cases: [(u8, u8, u8, u8, &str); 3] = [
            (0xFF, 0xFF, 0x88, 0x44, "rgba(255, 136, 68, 1.00)"),
            (0x80, 0xFF, 0x88, 0x44, "rgba(255, 136, 68, 0.50)"),
            (0x00, 0x00, 0x00, 0x00, "rgba(0, 0, 0, 0.00)"),
        ];
        for (a, r, g, b, expected) in cases {
            assert_eq!(expected, fmt_color(Color::new(a, r, g, b), "R"));
        }
    }

    #[test]
    fn color_to_string_r_lower_returns_rgb_without_alpha() {
        let cases: [(u8, u8, u8, u8, &str); 3] = [
            (0xFF, 0xFF, 0x88, 0x44, "rgb(255, 136, 68)"),
            (0x80, 0xFF, 0x88, 0x44, "rgb(255, 136, 68)"),
            (0xFF, 0x00, 0x00, 0x00, "rgb(0, 0, 0)"),
        ];
        for (a, r, g, b, expected) in cases {
            assert_eq!(expected, fmt_color(Color::new(a, r, g, b), "r"));
        }
    }

    #[test]
    fn hsl_color_to_string_r_upper_converts_to_rgba() {
        // Pure blue: HSL(240, 1, 0.5) = RGB(0, 0, 255)
        let hsl = HslColor::new(1.0, 240.0, 1.0, 0.5);
        assert_eq!("rgba(0, 0, 255, 1.00)", fmt_hsl(hsl, "R"));
    }

    #[test]
    fn hsv_color_to_string_r_lower_converts_to_rgb() {
        // Pure red: HSV(0, 1, 1) = RGB(255, 0, 0)
        let hsv = HsvColor::new(1.0, 0.0, 1.0, 1.0);
        assert_eq!("rgb(255, 0, 0)", fmt_hsv(hsv, "r"));
    }

    #[test]
    fn color_to_string_r_upper_pct_returns_rgba_percent() {
        let cases: [(u8, u8, u8, u8, &str); 3] = [
            (0xFF, 0xFF, 0x80, 0x00, "rgba(100%, 50%, 0%, 100%)"),
            (0x80, 0xFF, 0x80, 0x00, "rgba(100%, 50%, 0%, 50%)"),
            (0x00, 0x00, 0x00, 0x00, "rgba(0%, 0%, 0%, 0%)"),
        ];
        for (a, r, g, b, expected) in cases {
            assert_eq!(expected, fmt_color(Color::new(a, r, g, b), "R%"));
        }
    }

    #[test]
    fn color_to_string_r_lower_pct_returns_rgb_percent() {
        let cases: [(u8, u8, u8, u8, &str); 3] = [
            (0xFF, 0xFF, 0x80, 0x00, "rgb(100%, 50%, 0%)"),
            (0x80, 0xFF, 0x80, 0x00, "rgb(100%, 50%, 0%)"),
            (0xFF, 0x00, 0x00, 0x00, "rgb(0%, 0%, 0%)"),
        ];
        for (a, r, g, b, expected) in cases {
            assert_eq!(expected, fmt_color(Color::new(a, r, g, b), "r%"));
        }
    }

    #[test]
    fn hsl_color_to_string_r_upper_pct_converts_to_rgba_percent() {
        // Pure red: HSL(0, 1, 0.5) = RGB(255, 0, 0)
        let hsl = HslColor::new(1.0, 0.0, 1.0, 0.5);
        assert_eq!("rgba(100%, 0%, 0%, 100%)", fmt_hsl(hsl, "R%"));
    }

    #[test]
    fn hsl_color_to_string_l_upper_returns_hsla_with_alpha() {
        let cases: [(f64, f64, f64, f64, &str); 3] = [
            (1.0, 180.0, 0.5, 0.5, "hsla(180, 50%, 50%, 1.00)"),
            (0.5, 240.0, 0.8, 0.2, "hsla(240, 80%, 20%, 0.50)"),
            (0.0, 0.0, 0.0, 0.0, "hsla(0, 0%, 0%, 0.00)"),
        ];
        for (a, h, s, l, expected) in cases {
            assert_eq!(expected, fmt_hsl(HslColor::new(a, h, s, l), "L"));
        }
    }

    #[test]
    fn hsl_color_to_string_l_lower_returns_hsl_without_alpha() {
        let cases: [(f64, f64, f64, f64, &str); 3] = [
            (1.0, 180.0, 0.5, 0.5, "hsl(180, 50%, 50%)"),
            (0.5, 240.0, 0.8, 0.2, "hsl(240, 80%, 20%)"),
            (0.0, 0.0, 0.0, 0.0, "hsl(0, 0%, 0%)"),
        ];
        for (a, h, s, l, expected) in cases {
            assert_eq!(expected, fmt_hsl(HslColor::new(a, h, s, l), "l"));
        }
    }

    #[test]
    fn color_to_string_l_upper_converts_to_hsla() {
        // Pure red: RGB(255, 0, 0) = HSL(0, 100%, 50%)
        let color = Color::new(0xFF, 0xFF, 0x00, 0x00);
        assert_eq!("hsla(0, 100%, 50%, 1.00)", fmt_color(color, "L"));
    }

    #[test]
    fn color_to_string_l_lower_converts_to_hsl() {
        let color = Color::new(0xFF, 0xFF, 0x00, 0x00);
        assert_eq!("hsl(0, 100%, 50%)", fmt_color(color, "l"));
    }

    #[test]
    fn hsv_color_to_string_l_upper_converts_to_hsla() {
        // Pure red: HSV(0, 1, 1) = HSL(0, 100%, 50%)
        let hsv = HsvColor::new(1.0, 0.0, 1.0, 1.0);
        assert_eq!("hsla(0, 100%, 50%, 1.00)", fmt_hsv(hsv, "L"));
    }

    #[test]
    fn hsl_color_to_string_l_upper_pct_returns_hsla_all_percent() {
        let cases: [(f64, f64, f64, f64, &str); 3] = [
            (1.0, 180.0, 0.5, 0.5, "hsla(50%, 50%, 50%, 100%)"),
            (0.5, 90.0, 1.0, 1.0, "hsla(25%, 100%, 100%, 50%)"),
            (1.0, 0.0, 0.0, 0.0, "hsla(0%, 0%, 0%, 100%)"),
        ];
        for (a, h, s, l, expected) in cases {
            assert_eq!(expected, fmt_hsl(HslColor::new(a, h, s, l), "L%"));
        }
    }

    #[test]
    fn hsl_color_to_string_l_lower_pct_returns_hsl_all_percent() {
        let cases: [(f64, f64, f64, f64, &str); 3] = [
            (1.0, 180.0, 0.5, 0.5, "hsl(50%, 50%, 50%)"),
            (0.5, 90.0, 1.0, 1.0, "hsl(25%, 100%, 100%)"),
            (1.0, 0.0, 0.0, 0.0, "hsl(0%, 0%, 0%)"),
        ];
        for (a, h, s, l, expected) in cases {
            assert_eq!(expected, fmt_hsl(HslColor::new(a, h, s, l), "l%"));
        }
    }

    #[test]
    fn hsv_color_to_string_v_upper_returns_hsva_with_alpha() {
        let cases: [(f64, f64, f64, f64, &str); 3] = [
            (1.0, 180.0, 0.5, 0.5, "hsva(180, 50%, 50%, 1.00)"),
            (0.5, 240.0, 0.8, 0.2, "hsva(240, 80%, 20%, 0.50)"),
            (0.0, 0.0, 0.0, 0.0, "hsva(0, 0%, 0%, 0.00)"),
        ];
        for (a, h, s, v, expected) in cases {
            assert_eq!(expected, fmt_hsv(HsvColor::new(a, h, s, v), "V"));
        }
    }

    #[test]
    fn hsv_color_to_string_v_lower_returns_hsv_without_alpha() {
        let cases: [(f64, f64, f64, f64, &str); 3] = [
            (1.0, 180.0, 0.5, 0.5, "hsv(180, 50%, 50%)"),
            (0.5, 240.0, 0.8, 0.2, "hsv(240, 80%, 20%)"),
            (0.0, 0.0, 0.0, 0.0, "hsv(0, 0%, 0%)"),
        ];
        for (a, h, s, v, expected) in cases {
            assert_eq!(expected, fmt_hsv(HsvColor::new(a, h, s, v), "v"));
        }
    }

    #[test]
    fn color_to_string_v_upper_converts_to_hsva() {
        // Pure red: RGB(255, 0, 0) = HSV(0, 100%, 100%)
        let color = Color::new(0xFF, 0xFF, 0x00, 0x00);
        assert_eq!("hsva(0, 100%, 100%, 1.00)", fmt_color(color, "V"));
    }

    #[test]
    fn hsl_color_to_string_v_upper_converts_to_hsva() {
        // Pure red: HSL(0, 1, 0.5) = HSV(0, 100%, 100%)
        let hsl = HslColor::new(1.0, 0.0, 1.0, 0.5);
        assert_eq!("hsva(0, 100%, 100%, 1.00)", fmt_hsl(hsl, "V"));
    }

    #[test]
    fn hsv_color_to_string_v_upper_pct_returns_hsva_all_percent() {
        let cases: [(f64, f64, f64, f64, &str); 3] = [
            (1.0, 180.0, 0.5, 0.5, "hsva(50%, 50%, 50%, 100%)"),
            (0.5, 90.0, 1.0, 1.0, "hsva(25%, 100%, 100%, 50%)"),
            (1.0, 0.0, 0.0, 0.0, "hsva(0%, 0%, 0%, 100%)"),
        ];
        for (a, h, s, v, expected) in cases {
            assert_eq!(expected, fmt_hsv(HsvColor::new(a, h, s, v), "V%"));
        }
    }

    #[test]
    fn hsv_color_to_string_v_lower_pct_returns_hsv_all_percent() {
        let cases: [(f64, f64, f64, f64, &str); 3] = [
            (1.0, 180.0, 0.5, 0.5, "hsv(50%, 50%, 50%)"),
            (0.5, 90.0, 1.0, 1.0, "hsv(25%, 100%, 100%)"),
            (1.0, 0.0, 0.0, 0.0, "hsv(0%, 0%, 0%)"),
        ];
        for (a, h, s, v, expected) in cases {
            assert_eq!(expected, fmt_hsv(HsvColor::new(a, h, s, v), "v%"));
        }
    }

    #[test]
    fn color_to_string_invalid_format_throws() {
        let color = Color::new(0xFF, 0xFF, 0x00, 0x00);
        assert_eq!(
            color.to_string_format("Z").unwrap_err().message(),
            "Format string 'Z' is not supported."
        );
    }

    #[test]
    fn hsl_color_to_string_invalid_format_throws() {
        let color = HslColor::new(1.0, 0.0, 0.0, 0.0);
        assert!(color.to_string_format("Z").is_err());
    }

    #[test]
    fn hsv_color_to_string_invalid_format_throws() {
        let color = HsvColor::new(1.0, 0.0, 0.0, 0.0);
        assert!(color.to_string_format("Z").is_err());
    }

    #[test]
    fn color_to_string_reserved_and_removed_specifiers_throw() {
        for format in ["C", "c", "A", "a", "P", "h"] {
            let color = Color::new(0xFF, 0xFF, 0x00, 0x00);
            assert!(color.to_string_format(format).is_err(), "{format}");
        }
    }

    #[test]
    fn hsl_color_to_string_reserved_c_throws() {
        for format in ["C", "c"] {
            let color = HslColor::new(1.0, 0.0, 1.0, 0.5);
            assert!(color.to_string_format(format).is_err(), "{format}");
        }
    }

    #[test]
    fn hsv_color_to_string_reserved_c_throws() {
        for format in ["C", "c"] {
            let color = HsvColor::new(1.0, 0.0, 1.0, 1.0);
            assert!(color.to_string_format(format).is_err(), "{format}");
        }
    }

    // Additional coverage (not part of the reference suite).

    #[test]
    fn css_parsing_edge_cases() {
        // Too short for the css path and not a known color.
        assert_eq!(Color::try_parse("rgb(0,0,)"), None);
        assert_eq!(Color::try_parse("rgb(0,0,0,0,0)"), None);
        assert_eq!(Color::try_parse("rgb(256,0,0)"), None);
        assert_eq!(Color::try_parse("rgb(1e1,0,0)"), None);
        assert_eq!(Color::try_parse("rgb(-1,0,0)"), None);
        assert_eq!(
            Color::try_parse("  RGB( 1 , 2.0 , 3 )  "),
            None,
            "the prefix is checked before trimming"
        );
        assert_eq!(
            Color::try_parse("RGB( 1 , 2.0 , 3 )  "),
            Some(Color::new(255, 1, 2, 3))
        );
        assert_eq!(
            Color::try_parse("rgba(1,2,3,200%)"),
            Some(Color::new(255, 1, 2, 3))
        );
        assert_eq!(
            Color::try_parse("rgb(150%,0%,-10%)"),
            Some(Color::new(255, 255, 0, 0))
        );
        assert_eq!(Color::try_parse("#ggg"), None);
        assert_eq!(Color::try_parse("#"), None);
        assert_eq!(Color::try_parse("#\u{e9}\u{e9}\u{e9}"), None);
        assert_eq!(Color::try_parse("notacolor"), None);
        assert_eq!(
            Color::try_parse("transparent"),
            Some(Color::new(0, 255, 255, 255))
        );
    }

    #[test]
    fn uint32_round_trip_and_model_conversions() {
        let c = Color::from_uint32(0x40ff8844);
        assert_eq!(c, Color::new(0x40, 0xff, 0x88, 0x44));
        assert_eq!(c.to_uint32(), 0x40ff8844);
        assert_eq!(Color::from_rgb(1, 2, 3), Color::from_argb(255, 1, 2, 3));
        for value in [
            0xff000000u32,
            0xffffffff,
            0xff336699,
            0x80ff0000,
            0xff00ff00,
            0xff0000ff,
            0xff7f7f7f,
        ] {
            let color = Color::from_uint32(value);
            assert_eq!(color.to_hsl().to_rgb(), color, "{value:08x} via hsl");
            assert_eq!(color.to_hsv().to_rgb(), color, "{value:08x} via hsv");
        }
        assert_eq!(
            "#ff8844".parse::<Color>().unwrap(),
            Color::new(255, 255, 0x88, 0x44)
        );
    }
}
