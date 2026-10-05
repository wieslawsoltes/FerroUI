/// Defines how labels are positioned for command bar buttons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CommandBarDefaultLabelPosition {
    /// Label is displayed below the icon.
    #[default]
    Bottom = 0,
    /// Label is displayed to the right of the icon.
    Right = 1,
    /// No label is visible (icon only).
    Collapsed = 2,
}
