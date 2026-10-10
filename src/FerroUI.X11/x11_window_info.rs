//! What the event dispatcher knows about a window (the port of
//! `X11WindowInfo.cs`).

use crate::dispatching::x11_event_dispatcher::EventHandler;
use crate::x11_window::X11Window;
use std::rc::{Rc, Weak};

/// The handler of the events of a window of this connection, and the
/// window object when the window is a top-level of the backend.
#[derive(Clone)]
pub struct X11WindowInfo {
    event_handler: EventHandler,
    window: Option<Weak<X11Window>>,
}

impl X11WindowInfo {
    pub fn new(event_handler: EventHandler, window: Option<Weak<X11Window>>) -> Self {
        Self { event_handler, window }
    }

    pub fn event_handler(&self) -> &EventHandler {
        &self.event_handler
    }

    pub fn window(&self) -> Option<Rc<X11Window>> {
        self.window.as_ref().and_then(Weak::upgrade)
    }
}
