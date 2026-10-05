/// Defines which edge of the drawer page the drawer pane slides in from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DrawerPlacement {
    /// The drawer slides in from the left edge.
    Left = 0,

    /// The drawer slides in from the right edge.
    Right = 1,

    /// The drawer slides down from the top edge.
    Top = 2,

    /// The drawer slides up from the bottom edge.
    Bottom = 3,
}
