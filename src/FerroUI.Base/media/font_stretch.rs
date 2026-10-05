use std::fmt;
use std::str::FromStr;

use crate::utilities::FormatError;

/// Specifies the available font stretches (the OS/2 `usWidthClass` scale).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FontStretch {
    /// The normal font stretch.
    #[default]
    Normal = 5,
    /// The ultra-condensed font stretch.
    UltraCondensed = 1,
    /// The extra-condensed font stretch.
    ExtraCondensed = 2,
    /// The condensed font stretch.
    Condensed = 3,
    /// The semi-condensed font stretch.
    SemiCondensed = 4,
    /// The semi-expanded font stretch.
    SemiExpanded = 6,
    /// The expanded font stretch.
    Expanded = 7,
    /// The extra-expanded font stretch.
    ExtraExpanded = 8,
    /// The ultra-expanded font stretch.
    UltraExpanded = 9,
}

impl FontStretch {
    const NAMES: [(&'static str, FontStretch); 9] = [
        ("Normal", FontStretch::Normal),
        ("UltraCondensed", FontStretch::UltraCondensed),
        ("ExtraCondensed", FontStretch::ExtraCondensed),
        ("Condensed", FontStretch::Condensed),
        ("SemiCondensed", FontStretch::SemiCondensed),
        ("SemiExpanded", FontStretch::SemiExpanded),
        ("Expanded", FontStretch::Expanded),
        ("ExtraExpanded", FontStretch::ExtraExpanded),
        ("UltraExpanded", FontStretch::UltraExpanded),
    ];

    /// The stretch for a width class in `1..=9` (C# `(FontStretch)value`).
    pub fn from_i32(value: i32) -> Option<Self> {
        Self::NAMES.iter().map(|(_, stretch)| *stretch).find(|stretch| *stretch as i32 == value)
    }

    /// Parses a stretch name, ignoring case (C# `Enum.TryParse(token, true, ..)`).
    pub fn try_parse_ignore_case(s: &str) -> Option<Self> {
        Self::NAMES.iter().find(|(name, _)| name.eq_ignore_ascii_case(s)).map(|(_, value)| *value)
    }
}

impl fmt::Display for FontStretch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl FromStr for FontStretch {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        Self::try_parse_ignore_case(s)
            .or_else(|| s.parse::<i32>().ok().and_then(Self::from_i32))
            .ok_or_else(|| FormatError::from_string(format!("Invalid font stretch: '{s}'.")))
    }
}
