// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Represents a logical dock capability that can be controlled by policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DockCapability {
    /// Controls close operations.
    #[default]
    Close = 0,
    /// Controls pin and unpin operations.
    Pin = 1,
    /// Controls floating operations.
    Float = 2,
    /// Controls drag source behavior.
    Drag = 3,
    /// Controls drop target behavior.
    Drop = 4,
    /// Controls docking a window as a document.
    DockAsDocument = 5,
}

impl TryFrom<i32> for DockCapability {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Close),
            1 => Ok(Self::Pin),
            2 => Ok(Self::Float),
            3 => Ok(Self::Drag),
            4 => Ok(Self::Drop),
            5 => Ok(Self::DockAsDocument),
            other => Err(other),
        }
    }
}

impl DockCapability {
    /// Original declaration for native metadata generation without a Domain UI dependency.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("dock_capability.rs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(DockCapability::Close as i32, 0);
        assert_eq!(DockCapability::try_from(0), Ok(DockCapability::Close));
        assert_eq!(DockCapability::Pin as i32, 1);
        assert_eq!(DockCapability::try_from(1), Ok(DockCapability::Pin));
        assert_eq!(DockCapability::Float as i32, 2);
        assert_eq!(DockCapability::try_from(2), Ok(DockCapability::Float));
        assert_eq!(DockCapability::Drag as i32, 3);
        assert_eq!(DockCapability::try_from(3), Ok(DockCapability::Drag));
        assert_eq!(DockCapability::Drop as i32, 4);
        assert_eq!(DockCapability::try_from(4), Ok(DockCapability::Drop));
        assert_eq!(DockCapability::DockAsDocument as i32, 5);
        assert_eq!(
            DockCapability::try_from(5),
            Ok(DockCapability::DockAsDocument)
        );
        assert_eq!(DockCapability::default() as i32, 0);
        assert_eq!(DockCapability::try_from(999), Err(999));
    }
}
