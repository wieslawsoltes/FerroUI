//! Takes the events of the connection and hands each to the handler of
//! its window (the port of `X11EventDispatcher.cs`).

use crate::x11_platform::FerroX11Platform;
use crate::x11_structs::XEventName;
use crate::xlib::{self, XDisplay, XEvent};
use ferroui_base::threading::CancellationToken;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The handler of the events of one window (`EventHandler`).
pub type EventHandler = Rc<dyn Fn(&mut XEvent)>;

/// Sees every event before the windows do (`IEventHook`).
pub trait IEventHook {
    /// Whether the hook handled the event, which then goes no further.
    fn try_handle_event(&self, evt: &XEvent) -> bool;
}

/// Reads the connection of the UI thread.
pub struct X11EventDispatcher {
    platform: Weak<FerroX11Platform>,
    display: XDisplay,
    fd: i32,
    event_hook: RefCell<Option<Rc<dyn IEventHook>>>,
}

/// Releases the data of a generic event when the handling of the event
/// ends, however it ends.
struct EventDataGuard<'a> {
    display: XDisplay,
    event: &'a mut XEvent,
    has_data: bool,
}

impl Drop for EventDataGuard<'_> {
    fn drop(&mut self) {
        if self.has_data {
            xlib::x_free_event_data(self.display, self.event);
        }
    }
}

impl X11EventDispatcher {
    pub fn new(platform: &Rc<FerroX11Platform>) -> Rc<Self> {
        let display = platform.display();
        Rc::new(Self {
            platform: Rc::downgrade(platform),
            display,
            fd: xlib::x_connection_number(display),
            event_hook: RefCell::new(None),
        })
    }

    /// The file descriptor of the connection.
    pub fn fd(&self) -> i32 {
        self.fd
    }

    /// Whether events wait to be taken.
    pub fn is_pending(&self) -> bool {
        xlib::x_pending(self.display) != 0
    }

    pub fn event_hook(&self) -> Option<Rc<dyn IEventHook>> {
        self.event_hook.borrow().clone()
    }

    pub fn set_event_hook(&self, value: Option<Rc<dyn IEventHook>>) {
        let previous = self.event_hook.replace(value);
        drop(previous);
    }

    pub fn dispatch_x11_events(&self, cancellation_token: &CancellationToken) {
        let Some(platform) = self.platform.upgrade() else {
            return;
        };
        while self.is_pending() {
            if cancellation_token.is_cancellation_requested() {
                return;
            }

            let mut xev = xlib::x_next_event(self.display);
            if xlib::x_filter_event(&mut xev) {
                continue;
            }

            let is_generic = xlib::event_type(&xev) == XEventName::GenericEvent as i32;
            let has_data = is_generic && xlib::x_get_event_data(self.display, &mut xev);
            let guard = EventDataGuard { display: self.display, event: &mut xev, has_data };
            let xev = &mut *guard.event;

            if self.event_hook().is_some_and(|hook| hook.try_handle_event(xev)) {
                continue;
            }

            if is_generic {
                let cookie = *xlib::generic_event_cookie(xev);
                if let Some(xi2) = platform.xi2() {
                    if platform.info().x_input_opcode() == cookie.extension {
                        xi2.on_event(&cookie);
                    }
                }
            } else if let Some(window_info) = platform.get_window(xlib::event_window(xev)) {
                (window_info.event_handler())(xev);
            }
        }
        self.flush();
    }

    pub fn flush(&self) {
        xlib::x_flush(self.display);
    }
}
