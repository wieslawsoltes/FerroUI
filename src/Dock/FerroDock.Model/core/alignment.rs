// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines dock alignment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Alignment {
    /// Unset alignment.
    Unset = -1,
    /// Left alignment.
    #[default]
    Left = 0,
    /// Bottom alignment.
    Bottom = 1,
    /// Right alignment.
    Right = 2,
    /// Top alignment.
    Top = 3,
}

impl TryFrom<i32> for Alignment {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            -1 => Ok(Self::Unset),
            0 => Ok(Self::Left),
            1 => Ok(Self::Bottom),
            2 => Ok(Self::Right),
            3 => Ok(Self::Top),
            other => Err(other),
        }
    }
}

impl Alignment {
    /// Original declaration for native metadata generation without a Domain UI dependency.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("alignment.rs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(Alignment::Unset as i32, -1);
        assert_eq!(Alignment::try_from(-1), Ok(Alignment::Unset));
        assert_eq!(Alignment::Left as i32, 0);
        assert_eq!(Alignment::try_from(0), Ok(Alignment::Left));
        assert_eq!(Alignment::Bottom as i32, 1);
        assert_eq!(Alignment::try_from(1), Ok(Alignment::Bottom));
        assert_eq!(Alignment::Right as i32, 2);
        assert_eq!(Alignment::try_from(2), Ok(Alignment::Right));
        assert_eq!(Alignment::Top as i32, 3);
        assert_eq!(Alignment::try_from(3), Ok(Alignment::Top));
        assert_eq!(Alignment::default() as i32, 0);
        assert_eq!(Alignment::try_from(999), Err(999));
    }
}
