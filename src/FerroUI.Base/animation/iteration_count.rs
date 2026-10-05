use crate::utilities::FormatError;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

/// Defines the valid modes for an [`IterationCount`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(i32)]
pub enum IterationType {
    #[default]
    Many = 0,
    Infinite = 1,
}

/// Determines the number of iterations of an animation. Also defines its
/// repeat type.
#[derive(Clone, Copy, Debug, Default)]
pub struct IterationCount {
    type_: IterationType,
    value: u64,
}

impl IterationCount {
    /// An instance that makes the animation loop forever.
    pub const INFINITE: IterationCount = IterationCount { type_: IterationType::Infinite, value: 0 };

    /// Creates a count of `value` iterations.
    pub const fn new(value: u64) -> Self {
        Self { type_: IterationType::Many, value }
    }

    /// Creates a count with an explicit repeat type.
    pub const fn with_type(value: u64, type_: IterationType) -> Self {
        Self { type_, value }
    }

    /// The repeat type.
    pub const fn repeat_type(&self) -> IterationType {
        self.type_
    }

    /// Whether the animation loops forever.
    pub const fn is_infinite(&self) -> bool {
        matches!(self.type_, IterationType::Infinite)
    }

    /// The number of iterations.
    pub const fn value(&self) -> u64 {
        self.value
    }

    /// Parses `"Infinite"` (case-insensitive) or a non-negative integer.
    pub fn parse(s: &str) -> Result<IterationCount, FormatError> {
        let s = s.to_uppercase();
        let s = s.trim();

        if s.ends_with("INFINITE") {
            Ok(Self::INFINITE)
        } else {
            if s.starts_with('-') {
                return Err(FormatError::from_string("IterationCount can't be a negative number.".to_string()));
            }

            let digits = s.strip_prefix('+').unwrap_or(s);
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err(FormatError::from_string(format!("Invalid IterationCount string: \"{s}\".")));
            }
            let value = digits
                .parse::<u64>()
                .map_err(|_| FormatError::from_string(format!("Invalid IterationCount string: \"{s}\".")))?;

            Ok(IterationCount::new(value))
        }
    }
}

impl PartialEq for IterationCount {
    fn eq(&self, other: &Self) -> bool {
        (self.is_infinite() && other.is_infinite()) || (self.value == other.value && self.type_ == other.type_)
    }
}

impl Eq for IterationCount {}

impl Hash for IterationCount {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Equal values hash equally: the count is ignored when infinite.
        if self.is_infinite() {
            IterationType::Infinite.hash(state);
        } else {
            self.value.hash(state);
            self.type_.hash(state);
        }
    }
}

impl fmt::Display for IterationCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_infinite() {
            f.write_str("Infinite")
        } else {
            write!(f, "{}", self.value)
        }
    }
}

impl FromStr for IterationCount {
    type Err = FormatError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        IterationCount::parse(s)
    }
}
