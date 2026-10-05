use ferroui_base::utilities::span_helpers::{parse_double, InvariantF64};
use ferroui_base::utilities::{FormatError, SpanStringTokenizer};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

/// Defines the valid units for a [`GridLength`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GridUnitType {
    /// The row or column is auto-sized to fit its content.
    #[default]
    Auto = 0,

    /// The row or column is sized in device independent pixels.
    Pixel = 1,

    /// The row or column is sized as a weighted proportion of available space.
    Star = 2,
}

/// Holds the width or height of a `Grid`'s column and row definitions.
///
/// The default value is an auto-sized length with a value of zero.
#[derive(Clone, Copy, Debug, Default)]
pub struct GridLength {
    type_: GridUnitType,
    value: f64,
}

impl GridLength {
    /// A length that indicates that a row or column should auto-size to fit
    /// its content.
    pub const AUTO: GridLength = GridLength {
        type_: GridUnitType::Auto,
        value: 0.0,
    };

    /// A length that indicates that a row or column should fill its content.
    pub const STAR: GridLength = GridLength {
        type_: GridUnitType::Star,
        value: 1.0,
    };

    /// Creates a length in device independent pixels.
    ///
    /// Panics if the value is negative, NaN or infinite.
    pub fn from_pixels(value: f64) -> Self {
        Self::new(value, GridUnitType::Pixel)
    }

    /// Creates a length with the given unit.
    ///
    /// Panics if the value is negative, NaN or infinite.
    pub fn new(value: f64, type_: GridUnitType) -> Self {
        if !Self::is_valid_value(value) {
            panic!("Invalid value");
        }

        Self { type_, value }
    }

    fn is_valid_value(value: f64) -> bool {
        !(value < 0.0 || value.is_nan() || value.is_infinite())
    }

    /// A length that indicates that a row or column should auto-size to fit
    /// its content.
    #[inline]
    pub const fn auto() -> Self {
        Self::AUTO
    }

    /// A length that indicates that a row or column should fill its content.
    #[inline]
    pub const fn star() -> Self {
        Self::STAR
    }

    /// The unit of the length.
    #[inline]
    pub const fn grid_unit_type(&self) -> GridUnitType {
        self.type_
    }

    /// Whether the length has a [`GridUnitType`] of `Pixel`.
    #[inline]
    pub fn is_absolute(&self) -> bool {
        self.type_ == GridUnitType::Pixel
    }

    /// Whether the length has a [`GridUnitType`] of `Auto`.
    #[inline]
    pub fn is_auto(&self) -> bool {
        self.type_ == GridUnitType::Auto
    }

    /// Whether the length has a [`GridUnitType`] of `Star`.
    #[inline]
    pub fn is_star(&self) -> bool {
        self.type_ == GridUnitType::Star
    }

    /// The length.
    #[inline]
    pub const fn value(&self) -> f64 {
        self.value
    }

    /// Parses a string to return a [`GridLength`].
    pub fn parse(s: &str) -> Result<GridLength, FormatError> {
        let s = s.to_uppercase();

        if s == "AUTO" {
            Ok(Self::AUTO)
        } else if let Some(value_string) = s.strip_suffix('*') {
            let value_string = value_string.trim();
            let value = if !value_string.is_empty() {
                Self::parse_value(value_string)?
            } else {
                1.0
            };
            Ok(GridLength {
                type_: GridUnitType::Star,
                value,
            })
        } else {
            let value = Self::parse_value(&s)?;
            Ok(GridLength {
                type_: GridUnitType::Pixel,
                value,
            })
        }
    }

    fn parse_value(s: &str) -> Result<f64, FormatError> {
        match parse_double(s) {
            Some(value) if Self::is_valid_value(value) => Ok(value),
            _ => Err(FormatError::invalid_input(s)),
        }
    }

    /// Parses a string to return a collection of [`GridLength`]s.
    pub fn parse_lengths(s: &str) -> Result<Vec<GridLength>, FormatError> {
        let mut result = Vec::new();

        SpanStringTokenizer::new(s).scope(|tokenizer| {
            while let Some(item) = tokenizer.try_read_string()? {
                result.push(Self::parse(item)?);
            }
            Ok(())
        })?;

        Ok(result)
    }
}

impl PartialEq for GridLength {
    fn eq(&self, other: &Self) -> bool {
        (self.is_auto() && other.is_auto()) || (self.value == other.value && self.type_ == other.type_)
    }
}

impl Hash for GridLength {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Auto lengths compare equal whatever their value is.
        if !self.is_auto() {
            // Positive and negative zero compare equal.
            (self.value + 0.0).to_bits().hash(state);
        }
        self.type_.hash(state);
    }
}

impl fmt::Display for GridLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_auto() {
            return f.write_str("Auto");
        }

        InvariantF64(self.value).fmt(f)?;
        if self.is_star() {
            f.write_str("*")?;
        }
        Ok(())
    }
}

impl FromStr for GridLength {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
