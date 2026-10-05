/// Defines the cross-platform role of a visual element for non-client
/// hit-testing. Used to mark elements as titlebar drag areas, resize grips,
/// etc.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum WindowDecorationsElementRole {
    /// No special role. The element is invisible to chrome hit-testing.
    #[default]
    None = 0,

    /// An interactive element that is part of the decorations chrome (e.g.,
    /// a caption button). Set by themes on decoration template elements.
    /// Input is passed through to the element rather than being intercepted
    /// for non-client actions.
    DecorationsElement = 1,

    /// An interactive element set by user code that should receive input
    /// even when overlapping chrome areas. Has the same effect as
    /// [`DecorationsElement`](Self::DecorationsElement) but is intended for
    /// use by application developers.
    User = 2,

    /// The element acts as a titlebar drag area. Clicking and dragging on
    /// this element initiates a platform window move.
    TitleBar = 3,

    /// Resize grip for the north (top) edge.
    ResizeN = 4,

    /// Resize grip for the south (bottom) edge.
    ResizeS = 5,

    /// Resize grip for the east (right) edge.
    ResizeE = 6,

    /// Resize grip for the west (left) edge.
    ResizeW = 7,

    /// Resize grip for the northeast corner.
    ResizeNE = 8,

    /// Resize grip for the northwest corner.
    ResizeNW = 9,

    /// Resize grip for the southeast corner.
    ResizeSE = 10,

    /// Resize grip for the southwest corner.
    ResizeSW = 11,

    /// The element acts as the window close button. On Win32, maps to the
    /// system close behavior. On other platforms, treated as an interactive
    /// decoration element.
    CloseButton = 12,

    /// The element acts as the window minimize button. On Win32, maps to
    /// the system minimize behavior. On other platforms, treated as an
    /// interactive decoration element.
    MinimizeButton = 13,

    /// The element acts as the window maximize/restore button. On Win32,
    /// maps to the system maximize behavior. On other platforms, treated as
    /// an interactive decoration element.
    MaximizeButton = 14,

    /// The element acts as the window fullscreen toggle button. Treated as
    /// an interactive decoration element on all platforms.
    FullScreenButton = 15,
}
