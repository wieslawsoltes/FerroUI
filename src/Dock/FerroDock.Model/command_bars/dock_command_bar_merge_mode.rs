// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines how a command bar merges with the host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DockCommandBarMergeMode {
    /// Replace the host command bar.
    #[default]
    Replace = 0,
    /// Append items or bars to the host.
    Append = 1,
    /// Merge items into a matching group.
    MergeByGroup = 2,
}

impl TryFrom<i32> for DockCommandBarMergeMode {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Replace),
            1 => Ok(Self::Append),
            2 => Ok(Self::MergeByGroup),
            other => Err(other),
        }
    }
}

impl DockCommandBarMergeMode {
    /// Original declaration text consumed by the UI-owned metadata bridge.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("dock_command_bar_merge_mode.rs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(DockCommandBarMergeMode::Replace as i32, 0);
        assert_eq!(
            DockCommandBarMergeMode::try_from(0),
            Ok(DockCommandBarMergeMode::Replace)
        );
        assert_eq!(DockCommandBarMergeMode::Append as i32, 1);
        assert_eq!(
            DockCommandBarMergeMode::try_from(1),
            Ok(DockCommandBarMergeMode::Append)
        );
        assert_eq!(DockCommandBarMergeMode::MergeByGroup as i32, 2);
        assert_eq!(
            DockCommandBarMergeMode::try_from(2),
            Ok(DockCommandBarMergeMode::MergeByGroup)
        );
        assert_eq!(DockCommandBarMergeMode::default() as i32, 0);
        assert_eq!(DockCommandBarMergeMode::try_from(999), Err(999));
    }
}
