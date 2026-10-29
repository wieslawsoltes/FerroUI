// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Identifies the command bar type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DockCommandBarKind {
    /// Menu bar.
    #[default]
    Menu = 0,
    /// Tool bar.
    ToolBar = 1,
    /// Ribbon bar.
    Ribbon = 2,
}

impl TryFrom<i32> for DockCommandBarKind {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Menu),
            1 => Ok(Self::ToolBar),
            2 => Ok(Self::Ribbon),
            other => Err(other),
        }
    }
}

impl DockCommandBarKind {
    /// Original declaration text consumed by the UI-owned metadata bridge.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("dock_command_bar_kind.rs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(DockCommandBarKind::Menu as i32, 0);
        assert_eq!(
            DockCommandBarKind::try_from(0),
            Ok(DockCommandBarKind::Menu)
        );
        assert_eq!(DockCommandBarKind::ToolBar as i32, 1);
        assert_eq!(
            DockCommandBarKind::try_from(1),
            Ok(DockCommandBarKind::ToolBar)
        );
        assert_eq!(DockCommandBarKind::Ribbon as i32, 2);
        assert_eq!(
            DockCommandBarKind::try_from(2),
            Ok(DockCommandBarKind::Ribbon)
        );
        assert_eq!(DockCommandBarKind::default() as i32, 0);
        assert_eq!(DockCommandBarKind::try_from(999), Err(999));
    }
}
