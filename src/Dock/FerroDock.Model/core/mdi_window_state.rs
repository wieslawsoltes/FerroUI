// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines MDI window state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum MdiWindowState {
    /// Normal window state.
    #[default]
    Normal = 0,
    /// Minimized window state.
    Minimized = 1,
    /// Maximized window state.
    Maximized = 2,
}

impl TryFrom<i32> for MdiWindowState {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Normal),
            1 => Ok(Self::Minimized),
            2 => Ok(Self::Maximized),
            other => Err(other),
        }
    }
}

impl MdiWindowState {
    /// Original declaration for native metadata generation without a model UI dependency.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("mdi_window_state.rs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(MdiWindowState::Normal as i32, 0);
        assert_eq!(MdiWindowState::try_from(0), Ok(MdiWindowState::Normal));
        assert_eq!(MdiWindowState::Minimized as i32, 1);
        assert_eq!(MdiWindowState::try_from(1), Ok(MdiWindowState::Minimized));
        assert_eq!(MdiWindowState::Maximized as i32, 2);
        assert_eq!(MdiWindowState::try_from(2), Ok(MdiWindowState::Maximized));
        assert_eq!(MdiWindowState::default() as i32, 0);
        assert_eq!(MdiWindowState::try_from(999), Err(999));
    }
}
