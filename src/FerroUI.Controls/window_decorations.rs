/// Determines window decorations (title bar, border, etc) for a window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum WindowDecorations {
    /// No decorations.
    None = 0,

    /// Window border without titlebar.
    BorderOnly = 1,

    /// Fully decorated (default).
    Full = 2,
}
