// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines vertical or horizontal orientation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Orientation {
    /// Horizontal orientation.
    #[default]
    Horizontal = 0,
    /// Vertical orientation.
    Vertical = 1,
}

impl TryFrom<i32> for Orientation {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Horizontal),
            1 => Ok(Self::Vertical),
            other => Err(other),
        }
    }
}

impl Orientation {
    /// Original declaration for UI metadata generation without a Domain UI dependency.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("orientation.rs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(Orientation::Horizontal as i32, 0);
        assert_eq!(Orientation::try_from(0), Ok(Orientation::Horizontal));
        assert_eq!(Orientation::Vertical as i32, 1);
        assert_eq!(Orientation::try_from(1), Ok(Orientation::Vertical));
        assert_eq!(Orientation::default() as i32, 0);
        assert_eq!(Orientation::try_from(999), Err(999));
    }
}
