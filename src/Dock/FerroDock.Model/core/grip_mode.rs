// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines grip mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GripMode {
    /// Visible.
    #[default]
    Visible = 0,
    /// Auto hide.
    AutoHide = 1,
    /// Hidden.
    Hidden = 2,
}

impl TryFrom<i32> for GripMode {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Visible),
            1 => Ok(Self::AutoHide),
            2 => Ok(Self::Hidden),
            other => Err(other),
        }
    }
}

impl GripMode {
    /// Original declaration for native metadata generation without a Domain UI dependency.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("grip_mode.rs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(GripMode::Visible as i32, 0);
        assert_eq!(GripMode::try_from(0), Ok(GripMode::Visible));
        assert_eq!(GripMode::AutoHide as i32, 1);
        assert_eq!(GripMode::try_from(1), Ok(GripMode::AutoHide));
        assert_eq!(GripMode::Hidden as i32, 2);
        assert_eq!(GripMode::try_from(2), Ok(GripMode::Hidden));
        assert_eq!(GripMode::default() as i32, 0);
        assert_eq!(GripMode::try_from(999), Err(999));
    }
}
