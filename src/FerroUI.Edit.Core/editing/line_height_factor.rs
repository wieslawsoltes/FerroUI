use std::fmt;

/// The minimum formatted row height as a multiple of the editor's text height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineHeightFactor(f64);

// The constructor excludes NaN, infinity and signed zero.
impl Eq for LineHeightFactor {}

impl LineHeightFactor {
    pub fn new(value: f64) -> Result<Self, InvalidLineHeightFactor> {
        if value.is_finite() && value > 0.0 {
            Ok(Self(value))
        } else {
            Err(InvalidLineHeightFactor)
        }
    }

    pub fn get(self) -> f64 {
        self.0
    }
}

impl Default for LineHeightFactor {
    fn default() -> Self {
        Self(1.16)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidLineHeightFactor;

impl fmt::Display for InvalidLineHeightFactor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("The line height factor must be finite and positive")
    }
}

impl std::error::Error for InvalidLineHeightFactor {}
