// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Identifies the the related dock contract property that changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DockingWindowStateProperty {
    /// The the related dock contract property.
    #[default]
    IsOpen = 0,
    /// The the related dock contract property.
    IsActive = 1,
    /// The the related dock contract property.
    IsSelected = 2,
    /// The the related dock contract property.
    DockingState = 3,
}

impl TryFrom<i32> for DockingWindowStateProperty {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::IsOpen),
            1 => Ok(Self::IsActive),
            2 => Ok(Self::IsSelected),
            3 => Ok(Self::DockingState),
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(DockingWindowStateProperty::IsOpen as i32, 0);
        assert_eq!(
            DockingWindowStateProperty::try_from(0),
            Ok(DockingWindowStateProperty::IsOpen)
        );
        assert_eq!(DockingWindowStateProperty::IsActive as i32, 1);
        assert_eq!(
            DockingWindowStateProperty::try_from(1),
            Ok(DockingWindowStateProperty::IsActive)
        );
        assert_eq!(DockingWindowStateProperty::IsSelected as i32, 2);
        assert_eq!(
            DockingWindowStateProperty::try_from(2),
            Ok(DockingWindowStateProperty::IsSelected)
        );
        assert_eq!(DockingWindowStateProperty::DockingState as i32, 3);
        assert_eq!(
            DockingWindowStateProperty::try_from(3),
            Ok(DockingWindowStateProperty::DockingState)
        );
        assert_eq!(DockingWindowStateProperty::default() as i32, 0);
        assert_eq!(DockingWindowStateProperty::try_from(999), Err(999));
    }
}
