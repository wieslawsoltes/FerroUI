use std::fmt;

/// Represents a Version16Dot16 value from OpenType font tables.
/// The high 16 bits represent the major version, and the low 16 bits represent
/// the minor version as a fraction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct FontVersion {
    /// The major version number.
    pub major: u16,
    /// The minor version number (as a fraction of 65536).
    pub minor: u16,
}

impl FontVersion {
    /// Creates the version from a raw 32-bit Version16Dot16 value.
    #[inline]
    pub const fn new(value: u32) -> Self {
        Self { major: (value >> 16) as u16, minor: (value & 0xFFFF) as u16 }
    }

    /// Creates the version from major and minor components.
    #[inline]
    pub const fn from_parts(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// Converts the version to a floating-point representation.
    #[inline]
    pub fn to_float(&self) -> f32 {
        self.major as f32 + (self.minor as f32 / 65536.0)
    }

    /// Returns the raw 32-bit Version16Dot16 value.
    #[inline]
    pub const fn to_uint32(&self) -> u32 {
        ((self.major as u32) << 16) | self.minor as u32
    }
}

impl fmt::Display for FontVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // For common fractional values, show them nicely (e.g. 2.5 instead of 2.5000076).
        if self.minor == 0 {
            return write!(f, "{}", self.major);
        }

        if self.minor == 0x8000 {
            return write!(f, "{}.5", self.major);
        }

        let text = format!("{:.6}", self.to_float());

        f.write_str(text.trim_end_matches('0').trim_end_matches('.'))
    }
}

impl From<FontVersion> for f32 {
    fn from(version: FontVersion) -> f32 {
        version.to_float()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_joins_the_raw_value() {
        let version = FontVersion::new(0x0001_8000);

        assert_eq!(version, FontVersion::from_parts(1, 0x8000));
        assert_eq!(version.to_uint32(), 0x0001_8000);
        assert_eq!(version.to_float(), 1.5);
        assert_eq!(f32::from(version), 1.5);
    }

    #[test]
    fn displays_common_fractions_nicely() {
        assert_eq!(FontVersion::new(0x0002_0000).to_string(), "2");
        assert_eq!(FontVersion::new(0x0002_8000).to_string(), "2.5");
        assert_eq!(FontVersion::new(0x0000_5000).to_string(), "0.3125");
        assert_eq!(FontVersion::new(0x0001_4000).to_string(), "1.25");
    }
}
