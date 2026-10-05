use std::fmt;
use std::str::FromStr;

use crate::utilities::FormatError;

/// Defines a set of predefined font weights.
///
/// As defined by the OpenType specification, a font weight is any value in the
/// range 1..=1000 (variable fonts and the OS/2 table produce values between the
/// named ones), so this is an open set: a number with named constants.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontWeight(pub i32);

#[allow(non_upper_case_globals)]
impl FontWeight {
    /// Specifies a "thin" font weight.
    pub const Thin: FontWeight = FontWeight(100);
    /// Specifies an "extra-light" font weight.
    pub const ExtraLight: FontWeight = FontWeight(200);
    /// Specifies an "ultra-light" font weight.
    pub const UltraLight: FontWeight = FontWeight::ExtraLight;
    /// Specifies a "light" font weight.
    pub const Light: FontWeight = FontWeight(300);
    /// Specifies a "semi-light" font weight.
    pub const SemiLight: FontWeight = FontWeight(350);
    /// Specifies a "normal" font weight.
    pub const Normal: FontWeight = FontWeight(400);
    /// Specifies a "regular" font weight.
    pub const Regular: FontWeight = FontWeight::Normal;
    /// Specifies a "medium" font weight.
    pub const Medium: FontWeight = FontWeight(500);
    /// Specifies a "demi-bold" font weight.
    pub const DemiBold: FontWeight = FontWeight::SemiBold;
    /// Specifies a "semi-bold" font weight.
    pub const SemiBold: FontWeight = FontWeight(600);
    /// Specifies a "bold" font weight.
    pub const Bold: FontWeight = FontWeight(700);
    /// Specifies an "extra-bold" font weight.
    pub const ExtraBold: FontWeight = FontWeight(800);
    /// Specifies an "ultra-bold" font weight.
    pub const UltraBold: FontWeight = FontWeight::ExtraBold;
    /// Specifies a "black" font weight.
    pub const Black: FontWeight = FontWeight(900);
    /// Specifies a "heavy" font weight.
    pub const Heavy: FontWeight = FontWeight::Black;
    /// Specifies a "solid" font weight.
    pub const Solid: FontWeight = FontWeight::Black;
    /// Specifies an "extra-black" font weight.
    pub const ExtraBlack: FontWeight = FontWeight(950);
    /// Specifies an "ultra-black" font weight.
    pub const UltraBlack: FontWeight = FontWeight(950);

    const NAMES: [(&'static str, FontWeight); 18] = [
        ("Thin", FontWeight::Thin),
        ("ExtraLight", FontWeight::ExtraLight),
        ("UltraLight", FontWeight::UltraLight),
        ("Light", FontWeight::Light),
        ("SemiLight", FontWeight::SemiLight),
        ("Normal", FontWeight::Normal),
        ("Regular", FontWeight::Regular),
        ("Medium", FontWeight::Medium),
        ("DemiBold", FontWeight::DemiBold),
        ("SemiBold", FontWeight::SemiBold),
        ("Bold", FontWeight::Bold),
        ("ExtraBold", FontWeight::ExtraBold),
        ("UltraBold", FontWeight::UltraBold),
        ("Black", FontWeight::Black),
        ("Heavy", FontWeight::Heavy),
        ("Solid", FontWeight::Solid),
        ("ExtraBlack", FontWeight::ExtraBlack),
        ("UltraBlack", FontWeight::UltraBlack),
    ];

    /// The numeric weight (C# `(int)weight`).
    #[inline]
    pub const fn value(self) -> i32 {
        self.0
    }

    /// Parses a weight name, ignoring case (C# `Enum.TryParse(token, true, ..)`
    /// restricted to names; numbers are handled by [`FromStr`]).
    pub fn try_parse_ignore_case(s: &str) -> Option<Self> {
        Self::NAMES.iter().find(|(name, _)| name.eq_ignore_ascii_case(s)).map(|(_, value)| *value)
    }

    fn name(self) -> Option<&'static str> {
        // The canonical name for values that have aliases.
        Some(match self.0 {
            100 => "Thin",
            200 => "ExtraLight",
            300 => "Light",
            350 => "SemiLight",
            400 => "Normal",
            500 => "Medium",
            600 => "SemiBold",
            700 => "Bold",
            800 => "ExtraBold",
            900 => "Black",
            950 => "ExtraBlack",
            _ => return None,
        })
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        FontWeight::Normal
    }
}

impl fmt::Display for FontWeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name() {
            Some(name) => f.write_str(name),
            None => write!(f, "{}", self.0),
        }
    }
}

impl fmt::Debug for FontWeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl FromStr for FontWeight {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if let Some(weight) = Self::try_parse_ignore_case(s) {
            return Ok(weight);
        }
        s.parse::<i32>()
            .map(FontWeight)
            .map_err(|_| FormatError::from_string(format!("Invalid font weight: '{s}'.")))
    }
}

impl From<i32> for FontWeight {
    fn from(value: i32) -> Self {
        FontWeight(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_and_parsing() {
        assert_eq!(FontWeight::Regular, FontWeight::Normal);
        assert_eq!(FontWeight::Heavy, FontWeight::Black);
        assert_eq!("bold".parse::<FontWeight>().unwrap(), FontWeight::Bold);
        assert_eq!("450".parse::<FontWeight>().unwrap(), FontWeight(450));
        assert_eq!(FontWeight::DemiBold.to_string(), "SemiBold");
        assert_eq!(FontWeight(450).to_string(), "450");
        assert!("x".parse::<FontWeight>().is_err());
    }
}
