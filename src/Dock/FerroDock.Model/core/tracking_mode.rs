// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines tracking mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TrackingMode {
    /// Visible mode.
    #[default]
    Visible = 0,
    /// Pinned mode.
    Pinned = 1,
    /// Tab mode.
    Tab = 2,
}

impl TrackingMode {
    /// Original declaration for native metadata generation without a Domain UI dependency.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("tracking_mode.rs");
}

impl TryFrom<i32> for TrackingMode {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Visible),
            1 => Ok(Self::Pinned),
            2 => Ok(Self::Tab),
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(TrackingMode::Visible as i32, 0);
        assert_eq!(TrackingMode::try_from(0), Ok(TrackingMode::Visible));
        assert_eq!(TrackingMode::Pinned as i32, 1);
        assert_eq!(TrackingMode::try_from(1), Ok(TrackingMode::Pinned));
        assert_eq!(TrackingMode::Tab as i32, 2);
        assert_eq!(TrackingMode::try_from(2), Ok(TrackingMode::Tab));
        assert_eq!(TrackingMode::default() as i32, 0);
        assert_eq!(TrackingMode::try_from(999), Err(999));
    }
}
