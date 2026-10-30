// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines the position of the SplitView pane.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SplitViewPanePlacement {
    /// The pane is shown to the left of content.
    #[default]
    Left = 0,
    /// The pane is shown to the right of content.
    Right = 1,
    /// The pane is shown above the content.
    Top = 2,
    /// The pane is shown below the content.
    Bottom = 3,
}

impl SplitViewPanePlacement {
    /// Original declaration text for build-only native metadata generation.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("split_view_pane_placement.rs");
}

impl TryFrom<i32> for SplitViewPanePlacement {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Left),
            1 => Ok(Self::Right),
            2 => Ok(Self::Top),
            3 => Ok(Self::Bottom),
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(SplitViewPanePlacement::Left as i32, 0);
        assert_eq!(
            SplitViewPanePlacement::try_from(0),
            Ok(SplitViewPanePlacement::Left)
        );
        assert_eq!(SplitViewPanePlacement::Right as i32, 1);
        assert_eq!(
            SplitViewPanePlacement::try_from(1),
            Ok(SplitViewPanePlacement::Right)
        );
        assert_eq!(SplitViewPanePlacement::Top as i32, 2);
        assert_eq!(
            SplitViewPanePlacement::try_from(2),
            Ok(SplitViewPanePlacement::Top)
        );
        assert_eq!(SplitViewPanePlacement::Bottom as i32, 3);
        assert_eq!(
            SplitViewPanePlacement::try_from(3),
            Ok(SplitViewPanePlacement::Bottom)
        );
        assert_eq!(SplitViewPanePlacement::default() as i32, 0);
        assert_eq!(SplitViewPanePlacement::try_from(999), Err(999));
    }
}
