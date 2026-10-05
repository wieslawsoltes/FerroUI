/// Defines the behavior of the drawer pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DrawerBehavior {
    /// The drawer adapts its display mode based on the current drawer
    /// layout behavior.
    Auto = 0,

    /// The drawer always opens as a flyout overlay, regardless of layout
    /// behavior.
    Flyout = 1,

    /// The drawer is permanently open and cannot be closed.
    ///
    /// Setting this value forces the `IsOpen` property of the drawer page
    /// to true.
    Locked = 2,

    /// The drawer is hidden and cannot be opened.
    ///
    /// Setting this value forces the `IsOpen` property of the drawer page
    /// to false. The `Closing` event of the drawer page is not raised.
    Disabled = 3,
}
