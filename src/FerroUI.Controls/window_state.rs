/// Defines the minimized/maximized state of a window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum WindowState {
    /// The window is neither minimized or maximized.
    #[default]
    Normal = 0,

    /// The window is minimized.
    Minimized = 1,

    /// The window is maximized.
    Maximized = 2,

    /// The window is fullscreen.
    FullScreen = 3,
}
