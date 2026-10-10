//! A child window that takes the keyboard focus for its parent (the port
//! of `X11FocusProxy.cs`).

use crate::dispatching::EventHandler;
use crate::x11_platform::FerroX11Platform;
use crate::x11_structs::{CreateWindowArgs, EventMask, XEventName};
use crate::x11_window_info::X11WindowInfo;
use crate::xlib::{self, XDisplay, XEvent, XID};
use std::cell::Cell;
use std::rc::{Rc, Weak};

const INVISIBLE_BORDER: i32 = 0;
const DEPTH_COPY_FROM_PARENT: i32 = 0;
const OUT_OF_SCREEN: (i32, i32) = (-1, -1);
const SMALLEST: (i32, i32) = (1, 1);

/// An invisible window whose focus and key events go to its owner.
pub struct X11FocusProxy {
    handle: Cell<XID>,
    platform: Weak<FerroX11Platform>,
}

impl X11FocusProxy {
    pub(crate) fn new(platform: &Rc<FerroX11Platform>, parent: XID, event_handler: EventHandler) -> Rc<Self> {
        let handle = Self::prepare_x_window(platform.info().display(), parent);
        let this = Rc::new(Self { handle: Cell::new(handle), platform: Rc::downgrade(platform) });
        platform.set_window(
            handle,
            X11WindowInfo::new(Rc::new(move |ev: &mut XEvent| Self::on_event(&event_handler, ev)), None),
        );
        this
    }

    pub(crate) fn handle(&self) -> XID {
        self.handle.get()
    }

    pub(crate) fn cleanup(&self) {
        if self.handle.get() != 0 {
            if let Some(platform) = self.platform.upgrade() {
                platform.remove_window(self.handle.get());
            }
            self.handle.set(0);
        }
    }

    fn on_event(owner_event_handler: &EventHandler, ev: &mut XEvent) {
        let event_type = xlib::event_type(ev);
        if event_type == XEventName::FocusIn as i32 || event_type == XEventName::FocusOut as i32 {
            owner_event_handler(ev);
        }

        if event_type == XEventName::KeyPress as i32 || event_type == XEventName::KeyRelease as i32 {
            owner_event_handler(ev);
        }
    }

    fn prepare_x_window(display: XDisplay, parent: XID) -> XID {
        let value_mask = EventMask::FOCUS_CHANGE_MASK | EventMask::KEY_PRESS_MASK | EventMask::KEY_RELEASE_MASK;
        let mut attrs = xlib::new_set_window_attributes();
        // As the reference, which passes the event mask where the mask of
        // the attributes is expected.
        let handle = xlib::x_create_window(
            display,
            parent,
            OUT_OF_SCREEN.0,
            OUT_OF_SCREEN.1,
            SMALLEST.0,
            SMALLEST.1,
            INVISIBLE_BORDER,
            DEPTH_COPY_FROM_PARENT,
            CreateWindowArgs::InputOutput.0,
            std::ptr::null_mut(),
            value_mask.bits() as u32 as _,
            &mut attrs,
        );
        xlib::x_map_window(display, handle);
        xlib::x_select_input(display, handle, value_mask.bits() as u32 as _);
        handle
    }
}
