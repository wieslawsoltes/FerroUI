//! The hidden window that owns the windows without a taskbar button and is
//! the parent of embedded windows before they are placed.

use crate::simple_window::SimpleWindow;

/// The hidden parent window of the calling thread.
pub struct OffscreenParentWindow;

impl OffscreenParentWindow {
    /// The handle of the hidden parent window, which is created on first
    /// use and lives as long as the thread.
    pub fn handle() -> isize {
        thread_local! {
            static SIMPLE_WINDOW: SimpleWindow = SimpleWindow::new(None);
        }
        SIMPLE_WINDOW.with(SimpleWindow::handle)
    }
}
