// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines the available dock operations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DockOperation {
    /// None dock.
    #[default]
    None = 0,
    /// Fill dock.
    Fill = 1,
    /// Dock to left.
    Left = 2,
    /// Dock to bottom.
    Bottom = 3,
    /// Dock to right.
    Right = 4,
    /// Dock to top.
    Top = 5,
    /// Dock to window.
    Window = 6,
    /// Splits to the left of the region selected by the outer split policy.
    OuterLeft = 7,
    /// Splits above the region selected by the outer split policy.
    OuterTop = 8,
    /// Splits to the right of the region selected by the outer split policy.
    OuterRight = 9,
    /// Splits below the region selected by the outer split policy.
    OuterBottom = 10,
}

impl DockOperation {
    /// Native declaration text used by UI-owned metadata generation, without a UI dependency.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("dock_operation.rs");
}

impl TryFrom<i32> for DockOperation {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Fill),
            2 => Ok(Self::Left),
            3 => Ok(Self::Bottom),
            4 => Ok(Self::Right),
            5 => Ok(Self::Top),
            6 => Ok(Self::Window),
            7 => Ok(Self::OuterLeft),
            8 => Ok(Self::OuterTop),
            9 => Ok(Self::OuterRight),
            10 => Ok(Self::OuterBottom),
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(DockOperation::None as i32, 0);
        assert_eq!(DockOperation::try_from(0), Ok(DockOperation::None));
        assert_eq!(DockOperation::Fill as i32, 1);
        assert_eq!(DockOperation::try_from(1), Ok(DockOperation::Fill));
        assert_eq!(DockOperation::Left as i32, 2);
        assert_eq!(DockOperation::try_from(2), Ok(DockOperation::Left));
        assert_eq!(DockOperation::Bottom as i32, 3);
        assert_eq!(DockOperation::try_from(3), Ok(DockOperation::Bottom));
        assert_eq!(DockOperation::Right as i32, 4);
        assert_eq!(DockOperation::try_from(4), Ok(DockOperation::Right));
        assert_eq!(DockOperation::Top as i32, 5);
        assert_eq!(DockOperation::try_from(5), Ok(DockOperation::Top));
        assert_eq!(DockOperation::Window as i32, 6);
        assert_eq!(DockOperation::try_from(6), Ok(DockOperation::Window));
        assert_eq!(DockOperation::OuterLeft as i32, 7);
        assert_eq!(DockOperation::try_from(7), Ok(DockOperation::OuterLeft));
        assert_eq!(DockOperation::OuterTop as i32, 8);
        assert_eq!(DockOperation::try_from(8), Ok(DockOperation::OuterTop));
        assert_eq!(DockOperation::OuterRight as i32, 9);
        assert_eq!(DockOperation::try_from(9), Ok(DockOperation::OuterRight));
        assert_eq!(DockOperation::OuterBottom as i32, 10);
        assert_eq!(DockOperation::try_from(10), Ok(DockOperation::OuterBottom));
        assert_eq!(DockOperation::default() as i32, 0);
        assert_eq!(DockOperation::try_from(999), Err(999));
    }
}
