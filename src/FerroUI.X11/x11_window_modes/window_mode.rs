//! The base of the window modes (the port of
//! `X11WindowModes/WindowMode.cs`).
//!
//! The reference class keeps the window it belongs to (`Init`) and reads
//! the platform, the connection and the handles through it; here every
//! member takes the window, and the members with a body in the reference
//! are the default bodies of the trait.

use crate::x11_window::X11Window;
use crate::xlib::{Atom, XEvent, XID};
use ferroui_base::{PixelPoint, Point};
use std::rc::Rc;

/// How a window is shown, activated and placed.
pub trait X11WindowMode {
    /// Whether input to the window is blocked by its host.
    fn block_input(&self) -> bool {
        false
    }

    /// The mode as the mode of an embedded window, when it is one (the
    /// cast of the reference).
    fn x_embed(&self) -> Option<&super::XEmbedClientWindowMode> {
        None
    }

    /// Sees an event of the window first; whether it handled it.
    fn on_event(&self, _window: &X11Window, _ev: &mut XEvent) -> bool {
        false
    }

    fn activate(&self, _window: &X11Window) {}

    fn on_handle_created(&self, _window: &Rc<X11Window>, _handle: XID) {}

    fn on_destroy_notify(&self, _window: &X11Window) {}

    fn append_wm_protocols(&self, _window: &X11Window, _data: &mut Vec<Atom>) {}

    fn show(&self, window: &X11Window, _activate: bool, _is_dialog: bool) {
        window.set_shown(true);
    }

    fn point_to_screen(&self, window: &X11Window, pt: Point) -> PixelPoint;
    fn point_to_client(&self, window: &X11Window, pt: PixelPoint) -> Point;

    fn hide(&self, window: &X11Window) {
        window.set_shown(false);
    }
}
