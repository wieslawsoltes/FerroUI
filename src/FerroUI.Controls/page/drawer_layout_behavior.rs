/// Defines the layout behavior of the drawer page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DrawerLayoutBehavior {
    /// The drawer overlays the detail content when open.
    Overlay = 0,

    /// The drawer and detail content are shown side by side when open.
    Split = 1,

    /// A narrow rail strip is always visible; opening expands as an overlay
    /// over the detail content.
    CompactOverlay = 2,

    /// A narrow rail strip is always visible; opening expands and pushes
    /// the detail content aside.
    CompactInline = 3,
}
