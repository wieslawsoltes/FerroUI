//! Defines a color using the hue/saturation/lightness (HSL) model.
//!
//! Color conversion portions of this file are adapted from the Windows Community
//! Toolkit project (https://github.com/CommunityToolkit/WindowsCommunityToolkit),
//! MIT licensed.

use std::fmt;
use std::str::FromStr;

use crate::media::color::{split_components, strip_css_function, try_parse_double_or_percent};
use crate::media::{Color, HsvColor};
use crate::utilities::math_utilities::{self, MathUtilities};
use crate::utilities::span_helpers::{try_parse_double, FixedF2, InvariantF64, NumberStyles};
use crate::utilities::{CultureInfo, FormatError};

/// Defines a color using the hue/saturation/lightness (HSL) model.
/// This uses a cylindrical-coordinate representation of a color.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct HslColor {
    /// The Alpha (transparency) component in the range from 0..1 (percentage).
    /// 0 is fully transparent, 1 is fully opaque.
    pub a: f64,
    /// The Hue component in the range from 0..360 (degrees). This is the color's
    /// location, in degrees, on a color wheel/circle from 0 to 360. Note that 360
    /// is equivalent to 0 and will be adjusted automatically.
    pub h: f64,
    /// The Saturation component in the range from 0..1 (percentage). HSL
    /// saturation is not the same as HSV saturation.
    pub s: f64,
    /// The Lightness component in the range from 0..1 (percentage). 0 is fully
    /// black and 1 is fully white.
    pub l: f64,
}

#[inline]
fn round_to_byte(value: f64) -> u8 {
    value.round_ties_even() as u8
}

#[inline]
fn round_to_int(value: f64) -> i32 {
    value.round_ties_even() as i32
}

/// Brings a hue into `[0, 360)` by repeated addition/subtraction of 360.
///
/// A hue that cannot be brought into range that way (infinite, or so large that
/// subtracting 360 no longer changes it) is returned as is instead of looping forever.
//
// Deviation (DEVIATIONS.md, Colors): upstream's `while (hue >= 360.0) hue -= 360.0;`
// and `while (hue < 0.0) hue += 360.0;` never end for such a hue.
#[inline]
fn wrap_hue(mut hue: f64) -> f64 {
    while hue >= 360.0 {
        let next = hue - 360.0;
        if next == hue {
            break;
        }
        hue = next;
    }

    while hue < 0.0 {
        let next = hue + 360.0;
        if next == hue {
            break;
        }
        hue = next;
    }

    hue
}

#[inline]
fn clamp01(value: f64) -> f64 {
    let value = if value < 0.0 { 0.0 } else { value };
    if value > 1.0 {
        1.0
    } else {
        value
    }
}

impl HslColor {
    /// Initializes a new instance of the [`HslColor`] struct. Components are
    /// clamped to their ranges and a hue of 360 becomes 0.
    #[inline]
    pub fn new(alpha: f64, hue: f64, saturation: f64, lightness: f64) -> Self {
        Self::new_with_clamp(alpha, hue, saturation, lightness, true)
    }

    /// Initializes a new instance of the [`HslColor`] struct.
    ///
    /// This constructor exists only for internal use where performance is critical.
    /// Whether or not the channel values are in the correct range must be known:
    /// when `clamp_values` is `false` the components are stored as given.
    #[inline]
    pub fn new_with_clamp(
        alpha: f64,
        hue: f64,
        saturation: f64,
        lightness: f64,
        clamp_values: bool,
    ) -> Self {
        if clamp_values {
            let h = MathUtilities::clamp(hue, 0.0, 360.0);

            Self {
                a: MathUtilities::clamp(alpha, 0.0, 1.0),
                // The maximum value of Hue is technically 360 minus epsilon (359.999...).
                // A value of 360 itself wraps to zero.
                h: if h == 360.0 { 0.0 } else { h },
                s: MathUtilities::clamp(saturation, 0.0, 1.0),
                l: MathUtilities::clamp(lightness, 0.0, 1.0),
            }
        } else {
            Self {
                a: alpha,
                h: hue,
                s: saturation,
                l: lightness,
            }
        }
    }

    /// Initializes a new instance of the [`HslColor`] struct from an RGB color.
    #[inline]
    pub fn from_color(color: Color) -> Self {
        color.to_hsl()
    }

    #[inline]
    pub fn equals(&self, other: HslColor) -> bool {
        other.a == self.a && other.h == self.h && other.s == self.s && other.l == self.l
    }

    /// Returns the RGB color model equivalent of this HSL color.
    #[inline]
    pub fn to_rgb(&self) -> Color {
        HslColor::hsl_to_rgb(self.h, self.s, self.l, self.a)
    }

    /// Returns the HSV color model equivalent of this HSL color.
    #[inline]
    pub fn to_hsv(&self) -> HsvColor {
        HslColor::hsl_to_hsv(self.h, self.s, self.l, self.a)
    }

    /// Returns the string representation of the color using a format specifier.
    ///
    /// Supported format specifiers:
    /// - `L` / `l`: CSS `hsla(h, s%, l%, a)` / `hsl(h, s%, l%)`
    /// - `L%` / `l%`: `hsla(h%, s%, l%, a%)` / `hsl(h%, s%, l%)`
    /// - `X`, `x`, `H`, `R`, `r`, `R%`, `r%`: converted to RGB and formatted as such
    /// - `V`, `v`, `V%`, `v%`: converted to HSV and formatted as such
    ///
    /// A null or empty format is the same as `to_string()`. Any other format is an
    /// error. `format_provider` is ignored: color formatting is culture-invariant.
    pub fn to_string_format(
        &self,
        format: Option<&str>,
        format_provider: Option<&CultureInfo>,
    ) -> Result<String, FormatError> {
        let format = match format {
            Some(format) if !format.is_empty() => format,
            _ => return Ok(self.to_string()),
        };

        match format {
            "X" | "x" | "H" | "R" | "r" | "R%" | "r%" => self.to_rgb().to_string_format(Some(format), format_provider),
            "L" => Ok(self.format_hsl_css(true)),
            "l" => Ok(self.format_hsl_css(false)),
            "L%" => Ok(self.format_hsl_percent_css(true)),
            "l%" => Ok(self.format_hsl_percent_css(false)),
            "V" | "v" | "V%" | "v%" => self.to_hsv().to_string_format(Some(format), format_provider),
            _ => Err(FormatError::from_string(format!(
                "Format string '{format}' is not supported."
            ))),
        }
    }

    fn format_hsl_css(&self, include_alpha: bool) -> String {
        let h_deg = round_to_int(self.h);
        let s_pct = round_to_int(self.s * 100.0);
        let l_pct = round_to_int(self.l * 100.0);

        if include_alpha {
            return format!("hsla({h_deg}, {s_pct}%, {l_pct}%, {})", FixedF2(self.a));
        }

        format!("hsl({h_deg}, {s_pct}%, {l_pct}%)")
    }

    fn format_hsl_percent_css(&self, include_alpha: bool) -> String {
        let h_pct = round_to_int(self.h / 360.0 * 100.0);
        let s_pct = round_to_int(self.s * 100.0);
        let l_pct = round_to_int(self.l * 100.0);

        if include_alpha {
            let a_pct = round_to_int(self.a * 100.0);
            return format!("hsla({h_pct}%, {s_pct}%, {l_pct}%, {a_pct}%)");
        }

        format!("hsl({h_pct}%, {s_pct}%, {l_pct}%)")
    }

    /// Parses an HSL color string (`hsl(h, s, l)` or `hsla(h, s, l, a)`).
    pub fn parse(s: &str) -> Result<HslColor, FormatError> {
        match HslColor::try_parse(s) {
            Some(hsl_color) => Ok(hsl_color),
            None => Err(FormatError::from_string(format!(
                "Invalid HSL color string: '{s}'."
            ))),
        }
    }

    /// Parses an HSL color string (`hsl(h, s, l)` or `hsla(h, s, l, a)`),
    /// returning `None` when it is not valid. Saturation, lightness and alpha may
    /// be given as fractions or percentages.
    pub fn try_parse(s: &str) -> Option<HslColor> {
        let working = strip_css_function(s, "hsla(", "hsl(")?;
        let (components, count) = split_components(working);

        if count == 3 {
            // HSL
            let hue = try_parse_double(components[0], NumberStyles::NUMBER)?;
            let saturation = try_parse_double_or_percent(components[1])?;
            let lightness = try_parse_double_or_percent(components[2])?;

            Some(HslColor::new(1.0, hue, saturation, lightness))
        } else if count == 4 {
            // HSLA
            let hue = try_parse_double(components[0], NumberStyles::NUMBER)?;
            let saturation = try_parse_double_or_percent(components[1])?;
            let lightness = try_parse_double_or_percent(components[2])?;
            let alpha = try_parse_double_or_percent(components[3])?;

            Some(HslColor::new(alpha, hue, saturation, lightness))
        } else {
            None
        }
    }

    /// Creates an [`HslColor`] from alpha, hue, saturation and lightness (clamped).
    #[inline]
    pub fn from_ahsl(a: f64, h: f64, s: f64, l: f64) -> HslColor {
        HslColor::new(a, h, s, l)
    }

    /// Creates an opaque [`HslColor`] from hue, saturation and lightness (clamped).
    #[inline]
    pub fn from_hsl(h: f64, s: f64, l: f64) -> HslColor {
        HslColor::new(1.0, h, s, l)
    }

    /// Converts the given HSLA color component values to their RGB color equivalent.
    ///
    /// The hue is wrapped into `[0, 360)`; the other components are clamped to `0..1`.
    pub fn hsl_to_rgb(hue: f64, saturation: f64, lightness: f64, alpha: f64) -> Color {
        // Note: Conversion code is originally based on ColorHelper in the Windows
        // Community Toolkit (licensed MIT). It has been modified.

        // We want the hue to be between 0 and 359,
        // so we first ensure that that's the case.
        let hue = wrap_hue(hue);

        // We similarly clamp saturation, lightness and alpha between 0 and 1.
        let saturation = clamp01(saturation);
        let lightness = clamp01(lightness);
        let alpha = clamp01(alpha);

        let chroma = (1.0 - ((2.0 * lightness) - 1.0).abs()) * saturation;
        let h1 = hue / 60.0;
        let x = chroma * (1.0 - ((h1 % 2.0) - 1.0).abs());
        let m = lightness - (0.5 * chroma);
        let (r1, g1, b1);

        if h1 < 1.0 {
            r1 = chroma;
            g1 = x;
            b1 = 0.0;
        } else if h1 < 2.0 {
            r1 = x;
            g1 = chroma;
            b1 = 0.0;
        } else if h1 < 3.0 {
            r1 = 0.0;
            g1 = chroma;
            b1 = x;
        } else if h1 < 4.0 {
            r1 = 0.0;
            g1 = x;
            b1 = chroma;
        } else if h1 < 5.0 {
            r1 = x;
            g1 = 0.0;
            b1 = chroma;
        } else {
            r1 = chroma;
            g1 = 0.0;
            b1 = x;
        }

        Color::new(
            round_to_byte(255.0 * alpha),
            round_to_byte(255.0 * (r1 + m)),
            round_to_byte(255.0 * (g1 + m)),
            round_to_byte(255.0 * (b1 + m)),
        )
    }

    /// Converts the given HSLA color component values to their HSV color equivalent.
    pub fn hsl_to_hsv(hue: f64, saturation: f64, lightness: f64, alpha: f64) -> HsvColor {
        // We want the hue to be between 0 and 359,
        // so we first ensure that that's the case.
        let hue = wrap_hue(hue);

        // We similarly clamp saturation, lightness and alpha between 0 and 1.
        let saturation = clamp01(saturation);
        let lightness = clamp01(lightness);
        let alpha = clamp01(alpha);

        // The conversion algorithm is from the below link
        // https://en.wikipedia.org/wiki/HSL_and_HSV#Interconversion

        let v = lightness + (saturation * math_utilities::min(lightness, 1.0 - lightness));

        let s = if v <= 0.0 {
            0.0
        } else {
            2.0 * (1.0 - (lightness / v))
        };

        HsvColor::new(alpha, hue, s, v)
    }
}

/// Converts the HSL color to RGB (an explicit conversion in the reference API).
impl From<HslColor> for Color {
    #[inline]
    fn from(hsl_color: HslColor) -> Color {
        hsl_color.to_rgb()
    }
}

impl FromStr for HslColor {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        HslColor::parse(s)
    }
}

impl fmt::Display for HslColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "hsla({}, {}, {}, {})",
            InvariantF64(self.h),
            InvariantF64(self.s),
            InvariantF64(self.l),
            InvariantF64(self.a)
        )
    }
}

// Not from upstream: clamping, display, parse errors and conversions of edge
// values that the upstream suite (`color_tests.rs`) does not cover.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_clamps_and_wraps() {
        assert_eq!(
            HslColor::new(2.0, 360.0, -1.0, 5.0),
            HslColor::new_with_clamp(1.0, 0.0, 0.0, 1.0, false)
        );
        assert!(HslColor::new(1.0, f64::NAN, 0.0, 0.0).h.is_nan());
        assert_eq!(
            HslColor::from_hsl(10.0, 0.5, 0.5),
            HslColor::from_ahsl(1.0, 10.0, 0.5, 0.5)
        );
        assert_eq!(HslColor::default(), HslColor::new(0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn display_parse_and_conversions() {
        assert_eq!(
            HslColor::new(0.8, 200.0, 0.6, 0.4).to_string(),
            "hsla(200, 0.6, 0.4, 0.8)"
        );
        assert_eq!(
            HslColor::parse("nope").unwrap_err().message(),
            "Invalid HSL color string: 'nope'."
        );
        assert_eq!(
            "hsl(120, 100%, 50%)".parse::<HslColor>().unwrap(),
            HslColor::new(1.0, 120.0, 1.0, 0.5)
        );
        assert_eq!(HslColor::try_parse("hsl(120, 100%)"), None);
        assert_eq!(HslColor::try_parse("hsl(1, 2, 3, 4, 5)"), None);
        assert_eq!(
            Color::from(HslColor::new(1.0, 120.0, 1.0, 0.5)),
            Color::new(255, 0, 255, 0)
        );
        assert_eq!(
            HslColor::from_color(Color::new(255, 0, 0, 255)),
            HslColor::new(1.0, 240.0, 1.0, 0.5)
        );
        // Out-of-range hues wrap; non-finite hues terminate.
        assert_eq!(
            HslColor::hsl_to_rgb(480.0, 1.0, 0.5, 1.0),
            Color::new(255, 0, 255, 0)
        );
        assert_eq!(
            HslColor::hsl_to_rgb(-240.0, 1.0, 0.5, 1.0),
            Color::new(255, 0, 255, 0)
        );
        let _ = HslColor::hsl_to_rgb(f64::INFINITY, 1.0, 0.5, 1.0);
        let _ = HslColor::hsl_to_rgb(f64::NEG_INFINITY, 1.0, 0.5, 1.0);
        let _ = HslColor::hsl_to_rgb(1e300, 1.0, 0.5, 1.0);
    }
}
