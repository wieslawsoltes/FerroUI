use crate::utilities::FormatError;
use std::str::FromStr;

/// Determines the time index for a key frame: a fraction of the animation's
/// duration in `[0, 1]`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cue {
    cue_value: f64,
}

impl Cue {
    /// Creates a cue. Panics when the value is outside `[0, 1]`.
    pub fn new(value: f64) -> Self {
        match Self::try_new(value) {
            Some(cue) => cue,
            None => panic!("This cue object's value should be within or equal to 0.0 and 1.0"),
        }
    }

    /// Creates a cue; `None` when the value is outside `[0, 1]`.
    pub fn try_new(value: f64) -> Option<Self> {
        if (0.0..=1.0).contains(&value) {
            Some(Self { cue_value: value })
        } else {
            None
        }
    }

    /// The normalized percent value of the cue.
    #[inline]
    pub fn cue_value(&self) -> f64 {
        self.cue_value
    }

    /// Parses a cue from a percentage, with or without a trailing `%`.
    pub fn parse(value: &str) -> Result<Cue, FormatError> {
        let invalid = || FormatError::from_string(format!("Invalid Cue string \"{value}\""));
        let v = value.trim_end_matches('%');
        let res = crate::utilities::span_helpers::try_parse_double(v.trim(), crate::utilities::span_helpers::NumberStyles::FLOAT)
            .ok_or_else(invalid)?;
        Cue::try_new(res / 100.0).ok_or_else(|| {
            FormatError::from_string("This cue object's value should be within or equal to 0.0 and 1.0".to_string())
        })
    }

    /// Checks for value equality with a number.
    pub fn equals_f64(&self, other: f64) -> bool {
        self.cue_value == other
    }
}

impl FromStr for Cue {
    type Err = FormatError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Cue::parse(s)
    }
}
