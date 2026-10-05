/// Defines the visibility of the overflow button in a
/// [`CommandBar`](super::CommandBar).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CommandBarOverflowButtonVisibility {
    /// The overflow button is shown only when secondary commands are
    /// present.
    #[default]
    Auto = 0,
    /// The overflow button is always shown.
    Visible = 1,
    /// The overflow button is never shown.
    Collapsed = 2,
}
