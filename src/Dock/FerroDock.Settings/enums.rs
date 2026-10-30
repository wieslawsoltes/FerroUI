// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
/// The source `DockFloatingWindowOwnerPolicy` settings contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum DockFloatingWindowOwnerPolicy {
    /// Source `Default` value.
    #[default]
    Default = 0,
    /// Source `AlwaysOwned` value.
    AlwaysOwned = 1,
    /// Source `NeverOwned` value.
    NeverOwned = 2,
}
ferroui_base::ferro_markup_enum!(DockFloatingWindowOwnerPolicy {
    Default,
    AlwaysOwned,
    NeverOwned
});
/// The source `DockGlobalDockingPreset` settings contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum DockGlobalDockingPreset {
    /// Source `LocalFirst` value.
    #[default]
    LocalFirst = 0,
    /// Source `GlobalFirst` value.
    GlobalFirst = 1,
}
ferroui_base::ferro_markup_enum!(DockGlobalDockingPreset {
    LocalFirst,
    GlobalFirst
});
/// The source `DockCommandBarMergingScope` settings contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum DockCommandBarMergingScope {
    /// Source `ActiveDocument` value.
    #[default]
    ActiveDocument = 0,
    /// Source `ActiveDockable` value.
    ActiveDockable = 1,
}
ferroui_base::ferro_markup_enum!(DockCommandBarMergingScope {
    ActiveDocument,
    ActiveDockable
});
/// The source `WindowDragDockScope` settings contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum WindowDragDockScope {
    /// Source `FullWindow` value.
    #[default]
    FullWindow = 0,
    /// Source `WindowChrome` value.
    WindowChrome = 1,
}
ferroui_base::ferro_markup_enum!(WindowDragDockScope {
    FullWindow,
    WindowChrome
});

impl TryFrom<i32> for DockFloatingWindowOwnerPolicy {
    type Error = i32;
    fn try_from(value: i32) -> Result<Self, i32> {
        match value {
            0 => Ok(Self::Default),
            1 => Ok(Self::AlwaysOwned),
            2 => Ok(Self::NeverOwned),
            other => Err(other),
        }
    }
}

impl TryFrom<i32> for DockGlobalDockingPreset {
    type Error = i32;
    fn try_from(value: i32) -> Result<Self, i32> {
        match value {
            0 => Ok(Self::LocalFirst),
            1 => Ok(Self::GlobalFirst),
            other => Err(other),
        }
    }
}

impl TryFrom<i32> for DockCommandBarMergingScope {
    type Error = i32;
    fn try_from(value: i32) -> Result<Self, i32> {
        match value {
            0 => Ok(Self::ActiveDocument),
            1 => Ok(Self::ActiveDockable),
            other => Err(other),
        }
    }
}

impl TryFrom<i32> for WindowDragDockScope {
    type Error = i32;
    fn try_from(value: i32) -> Result<Self, i32> {
        match value {
            0 => Ok(Self::FullWindow),
            1 => Ok(Self::WindowChrome),
            other => Err(other),
        }
    }
}
