//! Defines a color using the hue/saturation/value (HSV) model.
//!
//! Color conversion portions of this file are adapted from the WinUI project
//! (https://github.com/microsoft/microsoft-ui-xaml), MIT licensed.

use std::fmt;
use std::str::FromStr;

use crate::media::color::{split_components, strip_css_function, try_parse_double_or_percent};
use crate::media::{Color, HslColor};
use crate::utilities::math_utilities::{self, MathUtilities};
use crate::utilities::span_helpers::{try_parse_double, FixedF2, InvariantF64, NumberStyles};
use crate::utilities::FormatError;

/// Defines a color using the hue/saturation/value (HSV) model.
/// This uses a cylindrical-coordinate representation of a color.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct HsvColor {
    /// The Alpha (transparency) component in the range from 0..1 (percentage).
    /// 0 is fully transparent, 1 is fully opaque.
    pub a: f64,
    /// The Hue component in the range from 0..360 (degrees). This is the color's
    /// location, in degrees, on a color wheel/circle from 0 to 360. Note that 360
    /// is equivalent to 0 and will be adjusted automatically.
    pub h: f64,
    /// The Saturation component in the range from 0..1 (percentage). 0 is a shade
    /// of gray (no color) and 1 is the full color.
    pub s: f64,
    /// The Value component in the range from 0..1 (percentage). 0 is fully black
    /// and 1 is the brightest.
    pub v: f64,
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

impl HsvColor {
    /// Initializes a new instance of the [`HsvColor`] struct. Components are
    /// clamped to their ranges and a hue of 360 becomes 0.
    #[inline]
    pub fn new(alpha: f64, hue: f64, saturation: f64, value: f64) -> Self {
        Self::new_with_clamp(alpha, hue, saturation, value, true)
    }

    /// Initializes a new instance of the [`HsvColor`] struct.
    ///
    /// This constructor exists only for internal use where performance is critical.
    /// Whether or not the channel values are in the correct range must be known:
    /// when `clamp_values` is `false` the components are stored as given.
    #[inline]
    pub fn new_with_clamp(
        alpha: f64,
        hue: f64,
        saturation: f64,
        value: f64,
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
                v: MathUtilities::clamp(value, 0.0, 1.0),
            }
        } else {
            Self {
                a: alpha,
                h: hue,
                s: saturation,
                v: value,
            }
        }
    }

    /// Initializes a new instance of the [`HsvColor`] struct from an RGB color.
    #[inline]
    pub fn from_color(color: Color) -> Self {
        color.to_hsv()
    }

    #[inline]
    pub fn equals(&self, other: HsvColor) -> bool {
        other.a == self.a && other.h == self.h && other.s == self.s && other.v == self.v
    }

    /// Returns the RGB color model equivalent of this HSV color.
    #[inline]
    pub fn to_rgb(&self) -> Color {
        HsvColor::hsv_to_rgb(self.h, self.s, self.v, self.a)
    }

    /// Returns the HSL color model equivalent of this HSV color.
    #[inline]
    pub fn to_hsl(&self) -> HslColor {
        HsvColor::hsv_to_hsl(self.h, self.s, self.v, self.a)
    }

    /// Returns the string representation of the color using a format specifier.
    ///
    /// Supported format specifiers:
    /// - `V` / `v`: `hsva(h, s%, v%, a)` / `hsv(h, s%, v%)`
    /// - `V%` / `v%`: `hsva(h%, s%, v%, a%)` / `hsv(h%, s%, v%)`
    /// - `X`, `x`, `H`, `R`, `r`, `R%`, `r%`: converted to RGB and formatted as such
    /// - `L`, `l`, `L%`, `l%`: converted to HSL and formatted as such
    ///
    /// An empty format is the same as `to_string()`. Any other format is an error.
    pub fn to_string_format(&self, format: &str) -> Result<String, FormatError> {
        if format.is_empty() {
            return Ok(self.to_string());
        }

        match format {
            "X" | "x" | "H" | "R" | "r" | "R%" | "r%" => self.to_rgb().to_string_format(format),
            "L" | "l" | "L%" | "l%" => self.to_hsl().to_string_format(format),
            "V" => Ok(self.format_hsv_css(true)),
            "v" => Ok(self.format_hsv_css(false)),
            "V%" => Ok(self.format_hsv_percent_css(true)),
            "v%" => Ok(self.format_hsv_percent_css(false)),
            _ => Err(FormatError::from_string(format!(
                "Format string '{format}' is not supported."
            ))),
        }
    }

    fn format_hsv_css(&self, include_alpha: bool) -> String {
        let h_deg = round_to_int(self.h);
        let s_pct = round_to_int(self.s * 100.0);
        let v_pct = round_to_int(self.v * 100.0);

        if include_alpha {
            return format!("hsva({h_deg}, {s_pct}%, {v_pct}%, {})", FixedF2(self.a));
        }

        format!("hsv({h_deg}, {s_pct}%, {v_pct}%)")
    }

    fn format_hsv_percent_css(&self, include_alpha: bool) -> String {
        let h_pct = round_to_int(self.h / 360.0 * 100.0);
        let s_pct = round_to_int(self.s * 100.0);
        let v_pct = round_to_int(self.v * 100.0);

        if include_alpha {
            let a_pct = round_to_int(self.a * 100.0);
            return format!("hsva({h_pct}%, {s_pct}%, {v_pct}%, {a_pct}%)");
        }

        format!("hsv({h_pct}%, {s_pct}%, {v_pct}%)")
    }

    /// Parses an HSV color string (`hsv(h, s, v)` or `hsva(h, s, v, a)`).
    pub fn parse(s: &str) -> Result<HsvColor, FormatError> {
        match HsvColor::try_parse(s) {
            Some(hsv_color) => Ok(hsv_color),
            None => Err(FormatError::from_string(format!(
                "Invalid HSV color string: '{s}'."
            ))),
        }
    }

    /// Parses an HSV color string (`hsv(h, s, v)` or `hsva(h, s, v, a)`),
    /// returning `None` when it is not valid. Saturation, value and alpha may be
    /// given as fractions or percentages.
    pub fn try_parse(s: &str) -> Option<HsvColor> {
        let working = strip_css_function(s, "hsva(", "hsv(")?;
        let (components, count) = split_components(working);

        if count == 3 {
            // HSV
            let hue = try_parse_double(components[0], NumberStyles::NUMBER)?;
            let saturation = try_parse_double_or_percent(components[1])?;
            let value = try_parse_double_or_percent(components[2])?;

            Some(HsvColor::new(1.0, hue, saturation, value))
        } else if count == 4 {
            // HSVA
            let hue = try_parse_double(components[0], NumberStyles::NUMBER)?;
            let saturation = try_parse_double_or_percent(components[1])?;
            let value = try_parse_double_or_percent(components[2])?;
            let alpha = try_parse_double_or_percent(components[3])?;

            Some(HsvColor::new(alpha, hue, saturation, value))
        } else {
            None
        }
    }

    /// Creates an [`HsvColor`] from alpha, hue, saturation and value (clamped).
    #[inline]
    pub fn from_ahsv(a: f64, h: f64, s: f64, v: f64) -> HsvColor {
        HsvColor::new(a, h, s, v)
    }

    /// Creates an opaque [`HsvColor`] from hue, saturation and value (clamped).
    #[inline]
    pub fn from_hsv(h: f64, s: f64, v: f64) -> HsvColor {
        HsvColor::new(1.0, h, s, v)
    }

    /// Converts the given HSVA color component values to their RGB color equivalent.
    ///
    /// The hue is wrapped into `[0, 360)`; the other components are clamped to `0..1`.
    pub fn hsv_to_rgb(hue: f64, saturation: f64, value: f64, alpha: f64) -> Color {
        // Note: Conversion code is originally based on the C++ in WinUI (licensed MIT)
        // https://github.com/microsoft/microsoft-ui-xaml/blob/main/dev/Common/ColorConversion.cpp
        // This was used because it is the best documented and likely most optimized for performance
        // Alpha support was added

        // We want the hue to be between 0 and 359,
        // so we first ensure that that's the case.
        let hue = wrap_hue(hue);

        // We similarly clamp saturation, value and alpha between 0 and 1.
        let saturation = clamp01(saturation);
        let value = clamp01(value);
        let alpha = clamp01(alpha);

        // The first thing that we need to do is to determine the chroma (see above for its definition).
        // Remember from above that:
        //
        // 1. The chroma is the difference between the maximum and the minimum of the RGB channels,
        // 2. The value is the maximum of the RGB channels, and
        // 3. The saturation comes from dividing the chroma by the maximum of the RGB channels (i.e., the value).
        //
        // From these facts, you can see that we can retrieve the chroma by simply multiplying the saturation and the value,
        // and we can retrieve the minimum of the RGB channels by subtracting the chroma from the value.
        let chroma = saturation * value;
        let min = value - chroma;

        // If the chroma is zero, then we have a greyscale color.  In that case, the maximum and the minimum RGB channels
        // have the same value (and, indeed, all of the RGB channels are the same), so we can just immediately return
        // the minimum value as the value of all the channels.
        if chroma == 0.0 {
            return Color::from_argb(
                round_to_byte(alpha * 255.0),
                round_to_byte(min * 255.0),
                round_to_byte(min * 255.0),
                round_to_byte(min * 255.0),
            );
        }

        // If the chroma is not zero, then we need to continue.  The first step is to figure out
        // what section of the color wheel we're located in.  In order to do that, we'll divide the hue by 60.
        // The resulting value means we're in one of the following locations:
        //
        // 0 - Between red and yellow.
        // 1 - Between yellow and green.
        // 2 - Between green and cyan.
        // 3 - Between cyan and blue.
        // 4 - Between blue and purple.
        // 5 - Between purple and red.
        //
        // In each of these sextants, one of the RGB channels is completely present, one is partially present, and one is not present.
        // For example, as we transition between red and yellow, red is completely present, green is becoming increasingly present, and blue is not present.
        // Then, as we transition from yellow and green, green is now completely present, red is becoming decreasingly present, and blue is still not present.
        // As we transition from green to cyan, green is still completely present, blue is becoming increasingly present, and red is no longer present.  And so on.
        //
        // To convert from hue to RGB value, we first need to figure out which of the three channels is in which configuration
        // in the sextant that we're located in.  Next, we figure out what value the completely-present color should have.
        // We know that chroma = (max - min), and we know that this color is the max color, so to find its value we simply add
        // min to chroma to retrieve max.  Finally, we consider how far we've transitioned from the pure form of that color
        // to the next color (e.g., how far we are from pure red towards yellow), and give a value to the partially present channel
        // equal to the minimum plus the chroma (i.e., the max minus the min), multiplied by the percentage towards the new color.
        // This gets us a value between the maximum and the minimum representing the partially present channel.
        // Finally, the not-present color must be equal to the minimum value, since it is the one least participating in the overall color.
        let sextant = (hue / 60.0) as i32;
        let intermediate_color_percentage = (hue / 60.0) - sextant as f64;
        let max = chroma + min;

        let mut r = 0.0;
        let mut g = 0.0;
        let mut b = 0.0;

        match sextant {
            0 => {
                r = max;
                g = min + (chroma * intermediate_color_percentage);
                b = min;
            }
            1 => {
                r = min + (chroma * (1.0 - intermediate_color_percentage));
                g = max;
                b = min;
            }
            2 => {
                r = min;
                g = max;
                b = min + (chroma * intermediate_color_percentage);
            }
            3 => {
                r = min;
                g = min + (chroma * (1.0 - intermediate_color_percentage));
                b = max;
            }
            4 => {
                r = min + (chroma * intermediate_color_percentage);
                g = min;
                b = max;
            }
            5 => {
                r = max;
                g = min;
                b = min + (chroma * (1.0 - intermediate_color_percentage));
            }
            _ => {}
        }

        Color::new(
            round_to_byte(alpha * 255.0),
            round_to_byte(r * 255.0),
            round_to_byte(g * 255.0),
            round_to_byte(b * 255.0),
        )
    }

    /// Converts the given HSVA color component values to their HSL color equivalent.
    pub fn hsv_to_hsl(hue: f64, saturation: f64, value: f64, alpha: f64) -> HslColor {
        // We want the hue to be between 0 and 359,
        // so we first ensure that that's the case.
        let hue = wrap_hue(hue);

        // We similarly clamp saturation, value and alpha between 0 and 1.
        let saturation = clamp01(saturation);
        let value = clamp01(value);
        let alpha = clamp01(alpha);

        // The conversion algorithm is from the below link
        // https://en.wikipedia.org/wiki/HSL_and_HSV#Interconversion

        let l = value * (1.0 - (saturation / 2.0));

        let s = if l <= 0.0 || l >= 1.0 {
            0.0
        } else {
            (value - l) / math_utilities::min(l, 1.0 - l)
        };

        HslColor::new(alpha, hue, s, l)
    }
}

/// Converts the HSV color to RGB (an explicit conversion in the reference API).
impl From<HsvColor> for Color {
    #[inline]
    fn from(hsv_color: HsvColor) -> Color {
        hsv_color.to_rgb()
    }
}

impl FromStr for HsvColor {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        HsvColor::parse(s)
    }
}

impl fmt::Display for HsvColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "hsva({}, {}, {}, {})",
            InvariantF64(self.h),
            InvariantF64(self.s),
            InvariantF64(self.v),
            InvariantF64(self.a)
        )
    }
}

#[cfg(test)]
mod tests {
    // The reference tests for this type live in the color test-suite (see `color.rs`).
    use super::*;

    #[test]
    fn construction_clamps_and_wraps() {
        assert_eq!(
            HsvColor::new(2.0, 360.0, -1.0, 5.0),
            HsvColor::new_with_clamp(1.0, 0.0, 0.0, 1.0, false)
        );
        assert!(HsvColor::new(1.0, f64::NAN, 0.0, 0.0).h.is_nan());
        assert_eq!(
            HsvColor::from_hsv(10.0, 0.5, 0.5),
            HsvColor::from_ahsv(1.0, 10.0, 0.5, 0.5)
        );
    }

    #[test]
    fn display_parse_and_conversions() {
        assert_eq!(
            HsvColor::new(0.8, 200.0, 0.6, 0.4).to_string(),
            "hsva(200, 0.6, 0.4, 0.8)"
        );
        assert_eq!(
            HsvColor::parse("nope").unwrap_err().message(),
            "Invalid HSV color string: 'nope'."
        );
        assert_eq!(
            "hsv(120, 100%, 100%)".parse::<HsvColor>().unwrap(),
            HsvColor::new(1.0, 120.0, 1.0, 1.0)
        );
        assert_eq!(
            Color::from(HsvColor::new(1.0, 120.0, 1.0, 1.0)),
            Color::new(255, 0, 255, 0)
        );
        assert_eq!(
            HsvColor::from_color(Color::new(255, 0, 0, 255)),
            HsvColor::new(1.0, 240.0, 1.0, 1.0)
        );
        // Every sextant.
        let expected = [
            (30.0, Color::new(255, 255, 128, 0)),
            (90.0, Color::new(255, 128, 255, 0)),
            (150.0, Color::new(255, 0, 255, 128)),
            (210.0, Color::new(255, 0, 128, 255)),
            (270.0, Color::new(255, 128, 0, 255)),
            (330.0, Color::new(255, 255, 0, 128)),
        ];
        for (hue, color) in expected {
            assert_eq!(HsvColor::hsv_to_rgb(hue, 1.0, 1.0, 1.0), color, "hue {hue}");
        }
        assert_eq!(
            HsvColor::hsv_to_rgb(480.0, 1.0, 1.0, 1.0),
            Color::new(255, 0, 255, 0)
        );
        let _ = HsvColor::hsv_to_rgb(f64::INFINITY, 1.0, 1.0, 1.0);
        let _ = HsvColor::hsv_to_rgb(-1e300, 1.0, 1.0, 1.0);
    }
}
