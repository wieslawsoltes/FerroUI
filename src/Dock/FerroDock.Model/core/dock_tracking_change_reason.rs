// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Indicates why global dock tracking state changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DockTrackingChangeReason {
    /// Unknown reason.
    #[default]
    Unknown = 0,
    /// The active dockable changed.
    ActiveDockableChanged = 1,
    /// The focused dockable changed.
    FocusedDockableChanged = 2,
    /// A window was activated.
    WindowActivated = 3,
    /// A window was deactivated.
    WindowDeactivated = 4,
    /// A dockable was activated.
    DockableActivated = 5,
    /// A dockable was deactivated.
    DockableDeactivated = 6,
    /// A window was closed.
    WindowClosed = 7,
    /// A window was removed.
    WindowRemoved = 8,
    /// A layout was initialized.
    LayoutInitialized = 9,
}

impl DockTrackingChangeReason {
    /// Source declaration used by the native metadata build generator.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("dock_tracking_change_reason.rs");
}

impl TryFrom<i32> for DockTrackingChangeReason {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::ActiveDockableChanged),
            2 => Ok(Self::FocusedDockableChanged),
            3 => Ok(Self::WindowActivated),
            4 => Ok(Self::WindowDeactivated),
            5 => Ok(Self::DockableActivated),
            6 => Ok(Self::DockableDeactivated),
            7 => Ok(Self::WindowClosed),
            8 => Ok(Self::WindowRemoved),
            9 => Ok(Self::LayoutInitialized),
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(DockTrackingChangeReason::Unknown as i32, 0);
        assert_eq!(
            DockTrackingChangeReason::try_from(0),
            Ok(DockTrackingChangeReason::Unknown)
        );
        assert_eq!(DockTrackingChangeReason::ActiveDockableChanged as i32, 1);
        assert_eq!(
            DockTrackingChangeReason::try_from(1),
            Ok(DockTrackingChangeReason::ActiveDockableChanged)
        );
        assert_eq!(DockTrackingChangeReason::FocusedDockableChanged as i32, 2);
        assert_eq!(
            DockTrackingChangeReason::try_from(2),
            Ok(DockTrackingChangeReason::FocusedDockableChanged)
        );
        assert_eq!(DockTrackingChangeReason::WindowActivated as i32, 3);
        assert_eq!(
            DockTrackingChangeReason::try_from(3),
            Ok(DockTrackingChangeReason::WindowActivated)
        );
        assert_eq!(DockTrackingChangeReason::WindowDeactivated as i32, 4);
        assert_eq!(
            DockTrackingChangeReason::try_from(4),
            Ok(DockTrackingChangeReason::WindowDeactivated)
        );
        assert_eq!(DockTrackingChangeReason::DockableActivated as i32, 5);
        assert_eq!(
            DockTrackingChangeReason::try_from(5),
            Ok(DockTrackingChangeReason::DockableActivated)
        );
        assert_eq!(DockTrackingChangeReason::DockableDeactivated as i32, 6);
        assert_eq!(
            DockTrackingChangeReason::try_from(6),
            Ok(DockTrackingChangeReason::DockableDeactivated)
        );
        assert_eq!(DockTrackingChangeReason::WindowClosed as i32, 7);
        assert_eq!(
            DockTrackingChangeReason::try_from(7),
            Ok(DockTrackingChangeReason::WindowClosed)
        );
        assert_eq!(DockTrackingChangeReason::WindowRemoved as i32, 8);
        assert_eq!(
            DockTrackingChangeReason::try_from(8),
            Ok(DockTrackingChangeReason::WindowRemoved)
        );
        assert_eq!(DockTrackingChangeReason::LayoutInitialized as i32, 9);
        assert_eq!(
            DockTrackingChangeReason::try_from(9),
            Ok(DockTrackingChangeReason::LayoutInitialized)
        );
        assert_eq!(DockTrackingChangeReason::default() as i32, 0);
        assert_eq!(DockTrackingChangeReason::try_from(999), Err(999));
    }
}
