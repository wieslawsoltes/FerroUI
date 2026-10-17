//! `GridLength.cs`: the width of a column or the height of a row, in the core's own type so that the
//! core does not depend on the controls.

use std::fmt;

/// The unit of a [`GridLength`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GridUnitType {
    /// The size is determined by the size of the content.
    #[default]
    Auto,
    /// The value is in device-independent pixels.
    Pixel,
    /// The value is a weighted share of the remaining space.
    Star,
}

/// A length that is a size in pixels, a weighted share of the remaining space (`*`) or determined
/// by the content (`Auto`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridLength {
    value: f64,
    grid_unit_type: GridUnitType,
}

impl GridLength {
    /// A length of `value` in `unit`.
    ///
    /// # Panics
    /// When `value` is not a number, infinite or negative (`ArgumentOutOfRangeException`).
    pub fn new(value: f64, unit: GridUnitType) -> Self {
        if value.is_nan() || value.is_infinite() || value < 0.0 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'value')");
        }
        Self { value, grid_unit_type: unit }
    }

    /// A length in pixels.
    pub fn pixels(value: f64) -> Self {
        Self::new(value, GridUnitType::Pixel)
    }

    /// `1*`.
    pub const STAR: GridLength = GridLength { value: 1.0, grid_unit_type: GridUnitType::Star };

    /// `Auto`.
    pub const AUTO: GridLength = GridLength { value: 1.0, grid_unit_type: GridUnitType::Auto };

    /// `Auto`.
    pub fn auto() -> Self {
        Self::AUTO
    }

    /// `1*`.
    pub fn star() -> Self {
        Self::STAR
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn grid_unit_type(&self) -> GridUnitType {
        self.grid_unit_type
    }

    pub fn is_auto(&self) -> bool {
        self.grid_unit_type == GridUnitType::Auto
    }

    pub fn is_star(&self) -> bool {
        self.grid_unit_type == GridUnitType::Star
    }

    pub fn is_absolute(&self) -> bool {
        self.grid_unit_type == GridUnitType::Pixel
    }

    /// Parses `Auto`, `<number>`, `<number>*` or `*` (invariant culture, case-insensitive `Auto`).
    ///
    /// # Panics
    /// When the text is not a length (`FormatException`), or is a negative length.
    pub fn parse(text: &str) -> Self {
        Self::try_parse(text).unwrap_or_else(|| panic!("The input string '{text}' was not in a correct format."))
    }

    /// [`parse`](Self::parse) without the panic of a malformed text.
    pub fn try_parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.eq_ignore_ascii_case("Auto") {
            return Some(Self::AUTO);
        }
        let star = text.ends_with('*');
        let number = if star { &text[..text.len() - 1] } else { text };
        let value = if number.is_empty() && star { 1.0 } else { number.trim().parse::<f64>().ok()? };
        Some(Self::new(value, if star { GridUnitType::Star } else { GridUnitType::Pixel }))
    }
}

impl Default for GridLength {
    /// `default(GridLength)`: 0 in the unit `Auto`, the first member of the enumeration.
    fn default() -> Self {
        Self { value: 0.0, grid_unit_type: GridUnitType::Auto }
    }
}

impl From<f64> for GridLength {
    fn from(value: f64) -> Self {
        Self::pixels(value)
    }
}

impl fmt::Display for GridLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_auto() {
            f.write_str("Auto")
        } else {
            write!(f, "{}{}", self.value, if self.is_star() { "*" } else { "" })
        }
    }
}
