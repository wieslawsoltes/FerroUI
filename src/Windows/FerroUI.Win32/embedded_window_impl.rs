//! A window that a host embeds into a window of its own: a child window
//! without decorations, created under the hidden parent window until the
//! host gives it its place.

use crate::interop::unmanaged_methods::{create_window_ex, WindowStyles};
use crate::offscreen_parent_window::OffscreenParentWindow;
use crate::window_impl::{WindowImpl, WindowKind, WindowProperties};
use ferroui_controls::{WindowDecorations, WindowState};
use std::rc::Rc;

/// An embeddable window of the Windows backend.
///
/// In the reference this is a class that derives from the window
/// implementation; here it is a window implementation of the embedded
/// kind, and this type creates it.
pub struct EmbeddedWindowImpl;

impl EmbeddedWindowImpl {
    /// Creates an embeddable window.
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Rc<WindowImpl> {
        WindowImpl::create(
            WindowKind::Embedded,
            WindowProperties {
                show_in_taskbar: false,
                is_resizable: false,
                is_minimizable: false,
                is_maximizable: false,
                decorations: WindowDecorations::None,
                is_full_screen: false,
                window_state: WindowState::Normal,
            },
        )
    }
}

pub(crate) fn create_window(atom: u16) -> isize {
    create_window_ex(0, atom, WindowStyles::WS_CHILD.bits(), 0, 0, 640, 480, OffscreenParentWindow::handle())
}
