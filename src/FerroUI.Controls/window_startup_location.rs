/// Determines the startup location of the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum WindowStartupLocation {
    /// The startup location is defined by the position property.
    #[default]
    Manual = 0,

    /// The startup location is the center of the screen.
    CenterScreen = 1,

    /// The startup location is the center of the owner window. If the owner
    /// window is not specified, the startup location will be manual.
    CenterOwner = 2,
}
