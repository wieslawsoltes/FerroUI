use ferroui_base::Size;

/// Specifies the reason for a window resize.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum WindowResizeReason {
    /// The resize reason is unknown or unspecified.
    #[default]
    Unspecified = 0,

    /// The resize was due to the user resizing the window, for example by
    /// dragging the window frame.
    User = 1,

    /// The resize was initiated by the application, for example by setting
    /// one of the sizing-related properties on the window.
    Application = 2,

    /// The resize was initiated by the layout system.
    Layout = 3,

    /// The resize was due to a change in DPI.
    DpiChange = 4,
}

/// Provides data for the window resized event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowResizedEventArgs {
    client_size: Size,
    reason: WindowResizeReason,
}

impl WindowResizedEventArgs {
    /// Creates the event args. Only the windowing classes raise the event.
    pub fn new(client_size: Size, reason: WindowResizeReason) -> Self {
        Self { client_size, reason }
    }

    /// The new client size of the window in device-independent pixels.
    pub fn client_size(&self) -> Size {
        self.client_size
    }

    /// The reason for the resize.
    pub fn reason(&self) -> WindowResizeReason {
        self.reason
    }
}
