// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines how the SplitView pane is displayed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SplitViewDisplayMode {
    /// Pane is displayed next to content and does not auto-collapse.
    /// When closed, the pane is completely hidden.
    #[default]
    Inline = 0,
    /// Pane is displayed next to content. When closed, the pane shows
    /// a compact view based on the related dock contract.
    /// Does not auto-collapse.
    CompactInline = 1,
    /// Pane overlays the content when open.
    /// When closed, the pane is completely hidden.
    /// Auto-collapses when user interacts outside the pane.
    Overlay = 2,
    /// Pane overlays the content when open.
    /// When closed, the pane shows a compact view based on
    /// the related dock contract.
    /// Auto-collapses when user interacts outside the pane.
    CompactOverlay = 3,
}

impl SplitViewDisplayMode {
    /// Original declaration text for build-only native metadata generation.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("split_view_display_mode.rs");
}

impl TryFrom<i32> for SplitViewDisplayMode {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Inline),
            1 => Ok(Self::CompactInline),
            2 => Ok(Self::Overlay),
            3 => Ok(Self::CompactOverlay),
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(SplitViewDisplayMode::Inline as i32, 0);
        assert_eq!(
            SplitViewDisplayMode::try_from(0),
            Ok(SplitViewDisplayMode::Inline)
        );
        assert_eq!(SplitViewDisplayMode::CompactInline as i32, 1);
        assert_eq!(
            SplitViewDisplayMode::try_from(1),
            Ok(SplitViewDisplayMode::CompactInline)
        );
        assert_eq!(SplitViewDisplayMode::Overlay as i32, 2);
        assert_eq!(
            SplitViewDisplayMode::try_from(2),
            Ok(SplitViewDisplayMode::Overlay)
        );
        assert_eq!(SplitViewDisplayMode::CompactOverlay as i32, 3);
        assert_eq!(
            SplitViewDisplayMode::try_from(3),
            Ok(SplitViewDisplayMode::CompactOverlay)
        );
        assert_eq!(SplitViewDisplayMode::default() as i32, 0);
        assert_eq!(SplitViewDisplayMode::try_from(999), Err(999));
    }
}
