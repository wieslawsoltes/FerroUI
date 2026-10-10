//! A top-level window whose keyboard focus is taken by a proxy child (the
//! port of `X11WindowModes/InputProxyWindowMode.cs`).

use super::{DefaultTopLevelWindowMode, X11WindowMode};
use crate::x11_focus_proxy::X11FocusProxy;
use crate::x11_structs::{RevertTo, XEventName};
use crate::x11_window::X11Window;
use crate::xlib::{self, Atom, XEvent, XID};
use ferroui_base::{PixelPoint, Point};
use std::cell::RefCell;
use std::rc::Rc;

/// The mode of a window with a focus proxy.
#[derive(Default)]
pub struct InputProxyWindowMode {
    focus_proxy: RefCell<Option<Rc<X11FocusProxy>>>,
}

impl InputProxyWindowMode {
    fn focus_proxy_handle(&self) -> Option<XID> {
        self.focus_proxy.borrow().as_ref().map(|proxy| proxy.handle())
    }
}

impl X11WindowMode for InputProxyWindowMode {
    fn on_handle_created(&self, window: &Rc<X11Window>, handle: XID) {
        // `OnFocusProxyEvent` of the reference does nothing with the
        // events the proxy forwards.
        let proxy = X11FocusProxy::new(window.platform(), handle, Rc::new(|_: &mut XEvent| {}));
        window.set_wm_class_of(proxy.handle(), Some("FocusProxy"));
        *self.focus_proxy.borrow_mut() = Some(proxy);
    }

    fn on_event(&self, window: &X11Window, ev: &mut XEvent) -> bool {
        let x11 = window.x11();
        if xlib::event_type(ev) == XEventName::ClientMessage as i32 {
            let message = xlib::client_message_event(ev);
            if message.data.get_long(0) as Atom == x11.atoms().WM_TAKE_FOCUS {
                let proxy = self.focus_proxy_handle().expect("the focus proxy exists while the window does");
                xlib::x_set_input_focus(x11.display(), proxy, RevertTo::Parent as i32, message.data.get_long(1) as _);
            }
        }
        false
    }

    fn activate(&self, window: &X11Window) {
        DefaultTopLevelWindowMode::activate_with(window, || {
            if let Some(proxy) = self.focus_proxy_handle() {
                xlib::x_set_input_focus(window.x11().display(), proxy, 0, 0);
            }
        });
    }

    fn on_destroy_notify(&self, _window: &X11Window) {
        let proxy = self.focus_proxy.borrow_mut().take();
        if let Some(proxy) = proxy {
            proxy.cleanup();
        }
    }

    fn append_wm_protocols(&self, window: &X11Window, data: &mut Vec<Atom>) {
        data.push(window.x11().atoms().WM_TAKE_FOCUS);
    }

    fn show(&self, window: &X11Window, activate: bool, _is_dialog: bool) {
        DefaultTopLevelWindowMode::show_default(window, activate);
    }

    fn hide(&self, window: &X11Window) {
        DefaultTopLevelWindowMode::hide_default(window);
    }

    fn point_to_client(&self, window: &X11Window, point: PixelPoint) -> Point {
        DefaultTopLevelWindowMode::point_to_client_default(window, point)
    }

    fn point_to_screen(&self, window: &X11Window, point: Point) -> PixelPoint {
        DefaultTopLevelWindowMode::point_to_screen_default(window, point)
    }
}
