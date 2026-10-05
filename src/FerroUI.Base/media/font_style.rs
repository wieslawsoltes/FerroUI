use std::fmt;
use std::str::FromStr;

use crate::utilities::FormatError;

/// Defines the available font styles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FontStyle {
    /// A normal font.
    #[default]
    Normal = 0,
    /// An italic font.
    Italic = 1,
    /// An oblique font.
    Oblique = 2,
}

impl FontStyle {
    const NAMES: [(&'static str, FontStyle); 3] =
        [("Normal", FontStyle::Normal), ("Italic", FontStyle::Italic), ("Oblique", FontStyle::Oblique)];

    /// Parses a style name, ignoring case (C# `Enum.TryParse(token, true, ..)`).
    pub fn try_parse_ignore_case(s: &str) -> Option<Self> {
        Self::NAMES.iter().find(|(name, _)| name.eq_ignore_ascii_case(s)).map(|(_, value)| *value)
    }
}

impl fmt::Display for FontStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl FromStr for FontStyle {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_parse_ignore_case(s.trim())
            .ok_or_else(|| FormatError::from_string(format!("Invalid font style: '{s}'.")))
    }
}
