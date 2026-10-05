//! Represents a size in device pixels.

use std::fmt;
use std::str::FromStr;

use crate::utilities::{FormatError, SpanStringTokenizer};
use crate::{Size, Vector};

/// Represents a size in device pixels.
///
/// Conversions from floating point saturate at the `i32` range (NaN becomes 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PixelSize {
    /// The width.
    pub width: i32,
    /// The height.
    pub height: i32,
}

impl PixelSize {
    /// A size representing zero.
    pub const EMPTY: PixelSize = PixelSize::new(0, 0);

    const FROM_SIZE_CEILING_EPSILON: f64 = 1e-6;

    /// Initializes a new instance of the [`PixelSize`] structure.
    #[inline]
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    /// Gets the aspect ratio of the size.
    #[inline]
    pub fn aspect_ratio(&self) -> f64 {
        self.width as f64 / self.height as f64
    }

    /// Parses a [`PixelSize`] string (`"width, height"`).
    pub fn parse(s: &str) -> Result<Self, FormatError> {
        match Self::try_parse(s) {
            Some(result) => Ok(result),
            None => Err(FormatError::new("Invalid PixelSize.")),
        }
    }

    /// Tries to parse a [`PixelSize`] string. Every kind of malformed input yields
    /// `None` (the reference implementation raises a format error for some of them,
    /// e.g. trailing tokens; there is no such distinction here).
    pub fn try_parse(source: &str) -> Option<Self> {
        if source.is_empty() {
            return None;
        }

        SpanStringTokenizer::with_separator(source, ',', Some("Invalid PixelSize."))
            .scope(|t| {
                if let Some(w) = t.try_read_int32()? {
                    if let Some(h) = t.try_read_int32()? {
                        return Ok(Some(PixelSize::new(w, h)));
                    }
                }

                Ok(None)
            })
            .ok()
            .flatten()
    }

    /// Returns a boolean indicating whether the size is equal to the other given size.
    #[inline]
    pub const fn equals(&self, other: PixelSize) -> bool {
        self.width == other.width && self.height == other.height
    }

    /// Returns a new [`PixelSize`] with the same height and the specified width.
    #[inline]
    pub const fn with_width(&self, width: i32) -> PixelSize {
        PixelSize::new(width, self.height)
    }

    /// Returns a new [`PixelSize`] with the same width and the specified height.
    #[inline]
    pub const fn with_height(&self, height: i32) -> PixelSize {
        PixelSize::new(self.width, height)
    }

    /// Converts the [`PixelSize`] to a device-independent [`Size`] using the
    /// specified scaling factor.
    #[inline]
    pub fn to_size(&self, scale: f64) -> Size {
        Size::new(self.width as f64 / scale, self.height as f64 / scale)
    }

    /// Converts the [`PixelSize`] to a device-independent [`Size`] using the
    /// specified per-axis scaling factor.
    #[inline]
    pub fn to_size_vector(&self, scale: Vector) -> Size {
        Size::new(self.width as f64 / scale.x, self.height as f64 / scale.y)
    }

    /// Converts the [`PixelSize`] to a device-independent [`Size`] using the
    /// specified dots per inch (DPI).
    #[inline]
    pub fn to_size_with_dpi(&self, dpi: f64) -> Size {
        self.to_size(dpi / 96.0)
    }

    /// Converts the [`PixelSize`] to a device-independent [`Size`] using the
    /// specified per-axis dots per inch (DPI).
    #[inline]
    pub fn to_size_with_dpi_vector(&self, dpi: Vector) -> Size {
        self.to_size_vector(Vector::new(dpi.x / 96.0, dpi.y / 96.0))
    }

    /// Converts a [`Size`] to device pixels using the specified scaling factor,
    /// rounding up.
    #[inline]
    pub fn from_size(size: Size, scale: f64) -> PixelSize {
        PixelSize::new(
            (size.width * scale).ceil() as i32,
            (size.height * scale).ceil() as i32,
        )
    }

    /// A tolerant ceiling variant of [`from_size`](Self::from_size): a value within
    /// `1e-6` of an integer snaps to that integer, otherwise it is rounded up.
    /// This prevents floating-point artifacts (e.g. `24.0000001`) from producing an
    /// extra pixel while still guaranteeing the pixel size covers the logical size.
    #[inline]
    pub fn from_size_ceiling(size: Size, scale: f64) -> PixelSize {
        PixelSize::new(
            Self::ceil_with_epsilon(size.width * scale),
            Self::ceil_with_epsilon(size.height * scale),
        )
    }

    #[inline]
    fn ceil_with_epsilon(value: f64) -> i32 {
        let rounded = value.round_ties_even();
        if (value - rounded).abs() < Self::FROM_SIZE_CEILING_EPSILON {
            return rounded as i32;
        }
        value.ceil() as i32
    }

    /// Converts a [`Size`] to device pixels using the specified per-axis scaling
    /// factor, rounding up.
    #[inline]
    pub fn from_size_vector(size: Size, scale: Vector) -> PixelSize {
        PixelSize::new(
            (size.width * scale.x).ceil() as i32,
            (size.height * scale.y).ceil() as i32,
        )
    }

    /// Converts a [`Size`] to device pixels using the specified dots per inch (DPI).
    #[inline]
    pub fn from_size_with_dpi(size: Size, dpi: f64) -> PixelSize {
        PixelSize::from_size(size, dpi / 96.0)
    }

    /// Converts a [`Size`] to device pixels using the specified per-axis dots per inch (DPI).
    #[inline]
    pub fn from_size_with_dpi_vector(size: Size, dpi: Vector) -> PixelSize {
        PixelSize::from_size_vector(size, Vector::new(dpi.x / 96.0, dpi.y / 96.0))
    }
}

impl FromStr for PixelSize {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        PixelSize::parse(s)
    }
}

impl fmt::Display for PixelSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        // (source, expected, expected error message)
        let cases: [(&str, PixelSize, Option<&str>); 2] = [
            ("1024,768", PixelSize::new(1024, 768), None),
            ("1024x768", PixelSize::default(), Some("Invalid PixelSize.")),
        ];

        for (source, expected, exception) in cases {
            let mut error = None;
            let mut result = PixelSize::default();
            match PixelSize::parse(source) {
                Ok(value) => result = value,
                Err(e) => error = Some(e),
            }

            assert_eq!(exception, error.as_ref().map(|e| e.message()));
            assert_eq!(expected, result);
        }
    }

    #[test]
    fn try_parse() {
        let cases: [(&str, PixelSize); 2] = [
            ("1024,768", PixelSize::new(1024, 768)),
            ("1024x768", PixelSize::EMPTY),
        ];

        for (source, expected) in cases {
            let result = PixelSize::try_parse(source).unwrap_or(PixelSize::EMPTY);

            assert_eq!(expected, result);
        }
    }

    #[test]
    fn from_size_ceiling_computes_expected_pixels() {
        let cases: [(i32, f64, i32); 9] = [
            (10, 1.0, 10),
            (10, 1.25, 13),
            (10, 1.5, 15),
            (10, 1.75, 18),
            (10, 2.0, 20),
            (10, 1.125, 12),
            (8, 1.5, 12),
            (0, 1.5, 0),
            (1, 2.5, 3),
        ];

        for (logical, scale, expected) in cases {
            let pixel =
                PixelSize::from_size_ceiling(Size::new(logical as f64, logical as f64), scale);

            assert_eq!(expected, pixel.width);
            assert_eq!(expected, pixel.height);
        }
    }

    #[test]
    fn from_size_ceiling_snaps_when_within_epsilon() {
        for scale in [1.5, 2.0, 3.0] {
            // Pick a logical size where logical * scale is an exact integer; perturbing it by a tiny
            // amount in either direction must still produce that integer (no spurious +1 from ceiling).
            const LOGICAL: i32 = 10;
            let exact = LOGICAL as f64 * scale;
            let below = exact - 1e-9;
            let above = exact + 1e-9;
            let rounded_below = below / scale;
            let rounded_above = above / scale;

            let p1 = PixelSize::from_size_ceiling(Size::new(rounded_below, rounded_below), scale);
            let p2 = PixelSize::from_size_ceiling(Size::new(rounded_above, rounded_above), scale);

            assert_eq!(exact as i32, p1.width);
            assert_eq!(exact as i32, p2.width);
        }
    }

    // Additional coverage (not part of the reference suite).

    #[test]
    fn conversions_and_display() {
        assert_eq!(PixelSize::new(4, 2).aspect_ratio(), 2.0);
        assert_eq!(PixelSize::new(4, 2).to_string(), "4, 2");
        assert_eq!(PixelSize::try_parse(""), None);
        assert_eq!(PixelSize::try_parse("1,2,3"), None);
        assert_eq!(PixelSize::try_parse("1"), None);
        assert_eq!(
            PixelSize::from_size(Size::new(10.1, 10.0), 1.0),
            PixelSize::new(11, 10)
        );
        assert_eq!(
            PixelSize::from_size_with_dpi(Size::new(10.0, 10.0), 144.0),
            PixelSize::new(15, 15)
        );
        assert_eq!(PixelSize::new(15, 30).to_size(1.5), Size::new(10.0, 20.0));
        assert_eq!(
            PixelSize::new(15, 30).to_size_with_dpi(144.0),
            Size::new(10.0, 20.0)
        );
    }
}
