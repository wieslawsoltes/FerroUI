/// Defines constants for how the pane of a split view should display.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SplitViewDisplayMode {
    /// Pane is displayed next to content, and does not auto collapse when
    /// tapped outside.
    #[default]
    Inline = 0,
    /// Pane is displayed next to content. When collapsed, pane is still
    /// visible according to the compact pane length. Pane does not auto
    /// collapse when tapped outside.
    CompactInline = 1,
    /// Pane is displayed above content. Pane collapses when tapped outside.
    Overlay = 2,
    /// Pane is displayed above content. When collapsed, pane is still
    /// visible according to the compact pane length. Pane collapses when
    /// tapped outside.
    CompactOverlay = 3,
}
