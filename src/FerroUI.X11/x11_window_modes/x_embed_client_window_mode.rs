//! A window that is the client of an XEmbed embedder: a socket of another
//! toolkit holds it (the port of
//! `X11WindowModes/XEmbedClientWindowMode.cs`).

use super::X11WindowMode;
use crate::x11_structs::{EventMask, XEmbedMessage, XEventName};
use crate::x11_window::X11Window;
use crate::xlib::{self, PropertyMode, XEvent, XID};
use ferroui_base::input::{InputElement, KeyboardDevice};
use ferroui_base::{PixelPoint, PixelSize, PixelVector, Point, Ref, Visual, WeakRef};
use ferroui_controls::embedding::EmbeddableControlRoot;
use ferroui_controls::TopLevel;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

thread_local! {
    /// The windows of this mode by their identifier on the server: how the
    /// keyboard device's change of focus finds the mode of the top-level
    /// the focus is in (the reference casts the platform implementation
    /// of the top-level and its mode).
    static WINDOWS: RefCell<HashMap<XID, Weak<X11Window>>> = RefCell::new(HashMap::new());
    /// Whether the keyboard device is listened to (the static constructor
    /// of the reference).
    static LISTENING: Cell<bool> = const { Cell::new(false) };
}

/// The mode of an embedded window.
#[derive(Default)]
pub struct XEmbedClientWindowMode {
    focused_in_embedder: Cell<bool>,
    embedder_activated: Cell<bool>,
    disabled: Cell<bool>,
    current_embedder: Cell<XID>,
    suppress_configure_events: Cell<bool>,
    saved_focus: RefCell<Option<WeakRef<InputElement>>>,
}

/// What a message of the embedder changes (`OnXEmbedMessage`), apart from
/// the activation that follows from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EmbedState {
    pub focused_in_embedder: bool,
    pub embedder_activated: bool,
    pub disabled: bool,
    pub current_embedder: XID,
}

impl EmbedState {
    /// The state after a message, and whether the activation has to be
    /// updated.
    pub(crate) fn after(mut self, message: XEmbedMessage, data1: XID) -> (EmbedState, bool) {
        let update = match message {
            XEmbedMessage::EmbeddedNotify => {
                // Reset
                self.embedder_activated = false;
                self.focused_in_embedder = false;
                self.disabled = false;
                self.current_embedder = data1;
                true
            }
            XEmbedMessage::FocusIn => {
                self.focused_in_embedder = true;
                true
            }
            XEmbedMessage::FocusOut => {
                self.focused_in_embedder = false;
                true
            }
            XEmbedMessage::WindowActivate => {
                self.embedder_activated = true;
                true
            }
            XEmbedMessage::WindowDeactivate => {
                self.embedder_activated = false;
                true
            }
            XEmbedMessage::ModalityOn => {
                self.disabled = true;
                false
            }
            XEmbedMessage::ModalityOff => {
                self.disabled = false;
                false
            }
            _ => false,
        };
        (self, update)
    }

    pub(crate) fn active(&self) -> bool {
        self.focused_in_embedder && self.embedder_activated
    }
}

impl XEmbedClientWindowMode {
    pub fn new() -> Self {
        Self::ensure_listening();
        Self::default()
    }

    /// The static constructor of the reference: when the focus moves to an
    /// element of an embedded top-level, the element is remembered and the
    /// embedder is asked for the focus.
    fn ensure_listening() {
        if LISTENING.with(Cell::get) {
            return;
        }
        let Some(keyboard_device) = KeyboardDevice::instance() else {
            return;
        };
        LISTENING.with(|listening| listening.set(true));

        // The subscription lasts as long as the device.
        let _ = keyboard_device.property_changed(|property_name| {
            if property_name != "FocusedElement" {
                return;
            }
            let Some(focused) = KeyboardDevice::instance().and_then(|device| device.focused_element()) else {
                return;
            };
            let visual: Ref<Visual> = focused.clone().upcast();
            let Some(root) = TopLevel::get_top_level(Some(&visual)).and_then(|tl| tl.cast::<EmbeddableControlRoot>()) else {
                return;
            };
            let Some(handle) = root.try_get_platform_handle() else {
                return;
            };
            let window = WINDOWS.with(|windows| windows.borrow().get(&(handle.handle() as XID)).and_then(Weak::upgrade));
            let Some(window) = window else {
                return;
            };
            if let Some(xembed_mode) = window.mode().x_embed() {
                if xembed_mode.current_embedder.get() != 0 {
                    xembed_mode.set_saved_focus(Some(&focused));
                    xembed_mode.send_x_embed_message(&window, XEmbedMessage::RequestFocus, 0, 0, 0);
                }
            }
        });
    }

    fn root(window: &X11Window) -> Option<Ref<EmbeddableControlRoot>> {
        window.input_root_or_none()?.focus_root().cast::<EmbeddableControlRoot>()
    }

    pub fn scaling(&self, window: &X11Window) -> f64 {
        window.scaling_override().unwrap_or(1.0)
    }

    pub fn set_scaling(&self, window: &X11Window, value: f64) {
        window.set_scaling_override(Some(value));
    }

    fn saved_focus(&self) -> Option<Ref<InputElement>> {
        self.saved_focus.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn set_saved_focus(&self, value: Option<&Ref<InputElement>>) {
        *self.saved_focus.borrow_mut() = value.map(Ref::downgrade);
    }

    fn state(&self) -> EmbedState {
        EmbedState {
            focused_in_embedder: self.focused_in_embedder.get(),
            embedder_activated: self.embedder_activated.get(),
            disabled: self.disabled.get(),
            current_embedder: self.current_embedder.get(),
        }
    }

    fn set_state(&self, state: EmbedState) {
        self.focused_in_embedder.set(state.focused_in_embedder);
        self.embedder_activated.set(state.embedder_activated);
        self.disabled.set(state.disabled);
        self.current_embedder.set(state.current_embedder);
    }

    /// The embedder of the window; 0 without one.
    pub fn current_embedder(&self) -> XID {
        self.current_embedder.get()
    }

    fn send_x_embed_message(&self, window: &X11Window, message: XEmbedMessage, detail: i64, data1: i64, data2: i64) {
        let current_embedder = self.current_embedder.get();
        if current_embedder == 0 {
            return;
        }
        let x11 = window.x11();
        let mut xev = xlib::new_event();
        {
            let client_message = xlib::client_message_event_mut(&mut xev);
            client_message.type_ = XEventName::ClientMessage as i32;
            client_message.send_event = 1;
            client_message.window = current_embedder;
            client_message.message_type = x11.atoms()._XEMBED;
            client_message.format = 32;
            client_message.data.set_long(0, 0);
            client_message.data.set_long(1, message as i32 as _);
            client_message.data.set_long(2, detail as _);
            client_message.data.set_long(3, data1 as _);
            client_message.data.set_long(4, data2 as _);
        }
        xlib::x_send_event(x11.display(), current_embedder, false, EventMask::NO_EVENT_MASK.bits() as _, &mut xev);
    }

    fn reset(&self, window: &X11Window) {
        self.embedder_activated.set(false);
        self.focused_in_embedder.set(false);
        self.disabled.set(false);
        self.update_activation(window);
    }

    fn on_x_embed_message(&self, window: &X11Window, message: i32, data1: XID) {
        let Some(message) = XEmbedMessage::from_value(message) else {
            return;
        };
        let (state, update) = self.state().after(message, data1);
        if message == XEmbedMessage::EmbeddedNotify {
            // The reference resets, which updates the activation, and then takes the
            // embedder of the message.
            self.reset(window);
            self.set_state(state);
            return;
        }
        self.set_state(state);
        if update {
            self.update_activation(window);
        }
    }

    fn update_activation(&self, window: &X11Window) {
        let active = self.state().active();
        let root = Self::root(window);

        if active {
            if let Some(root) = &root {
                let scope: Ref<InputElement> = root.clone().upcast();
                root.focus_manager().set_focus_scope(&scope);
            }
            if let Some(saved_focus) = self.saved_focus() {
                saved_focus.focus();
            }
            self.set_saved_focus(None);
        } else {
            let focused = root
                .filter(|root| root.is_keyboard_focus_within())
                .and_then(|root| root.focus_manager().get_focused_element());
            self.set_saved_focus(focused.as_ref());
            window.raise_lost_focus();
        }
    }

    pub fn process_interactive_resize(&self, window: &X11Window, size: PixelSize) {
        self.suppress_configure_events.set(true);
        window.interactive_resize(size);
    }

    fn get_window_offset(window: &X11Window) -> PixelVector {
        let x11 = window.x11();
        let (offset_x, offset_y, _) =
            xlib::x_translate_coordinates(x11.display(), window.xid(), x11.default_root_window(), 0, 0).unwrap_or_default();
        PixelVector::new(offset_x, offset_y)
    }
}

impl X11WindowMode for XEmbedClientWindowMode {
    fn block_input(&self) -> bool {
        self.disabled.get()
    }

    fn x_embed(&self) -> Option<&XEmbedClientWindowMode> {
        Some(self)
    }

    fn on_handle_created(&self, window: &Rc<X11Window>, handle: XID) {
        let x11 = window.x11();
        let data = [0, 1 /* XEMBED_MAPPED */];

        xlib::x_change_property_longs(
            x11.display(),
            handle,
            x11.atoms()._XEMBED_INFO,
            x11.atoms()._XEMBED_INFO,
            PropertyMode::Replace,
            &data,
        );
        self.set_scaling(window, 1.0);
        WINDOWS.with(|windows| windows.borrow_mut().insert(handle, Rc::downgrade(window)));
    }

    fn on_destroy_notify(&self, window: &X11Window) {
        WINDOWS.with(|windows| windows.borrow_mut().remove(&window.xid()));
    }

    fn on_event(&self, window: &X11Window, ev: &mut XEvent) -> bool {
        let event_type = xlib::event_type(ev);
        // In this mode we are getting the expected size directly from the embedder
        if self.suppress_configure_events.get() && event_type == XEventName::ConfigureNotify as i32 {
            return true;
        }
        if event_type == XEventName::MapNotify as i32 {
            if let Some(root) = Self::root(window) {
                root.start_rendering();
            }
        } else if event_type == XEventName::UnmapNotify as i32 {
            if let Some(root) = Self::root(window) {
                root.stop_rendering();
            }
        } else if event_type == XEventName::ReparentNotify as i32 {
            if let Some(root) = Self::root(window) {
                root.stop_rendering();
            }
            self.current_embedder.set(0);
            self.reset(window);
        } else if event_type == XEventName::ClientMessage as i32 {
            let message = xlib::client_message_event(ev);
            if message.message_type == window.x11().atoms()._XEMBED {
                let (kind, data1) = (message.data.get_long(1) as i32, message.data.get_long(3) as XID);
                self.on_x_embed_message(window, kind, data1);
                return true;
            }
        }

        false
    }

    fn point_to_client(&self, window: &X11Window, point: PixelPoint) -> Point {
        let pos = Self::get_window_offset(window);
        Point::new((point.x - pos.x) as f64 / window.scaling(), (point.y - pos.y) as f64 / window.scaling())
    }

    fn point_to_screen(&self, window: &X11Window, point: Point) -> PixelPoint {
        PixelPoint::new((point.x * window.scaling()) as i32, (point.y * window.scaling()) as i32)
            + Self::get_window_offset(window)
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    const START: EmbedState =
        EmbedState { focused_in_embedder: false, embedder_activated: false, disabled: false, current_embedder: 0 };

    #[test]
    fn the_window_is_active_when_the_embedder_is_active_and_gave_it_the_focus() {
        let (state, update) = START.after(XEmbedMessage::EmbeddedNotify, 0x77);
        assert!(update && state.current_embedder == 0x77 && !state.active());
        let (state, update) = state.after(XEmbedMessage::WindowActivate, 0);
        assert!(update && !state.active());
        let (state, update) = state.after(XEmbedMessage::FocusIn, 0);
        assert!(update && state.active());
        let (state, _) = state.after(XEmbedMessage::WindowDeactivate, 0);
        assert!(!state.active() && state.focused_in_embedder);
        let (state, _) = state.after(XEmbedMessage::WindowActivate, 0);
        let (state, _) = state.after(XEmbedMessage::FocusOut, 0);
        assert!(!state.active() && state.embedder_activated);
    }

    #[test]
    fn modality_blocks_input_and_changes_no_activation() {
        let (state, update) = START.after(XEmbedMessage::ModalityOn, 0);
        assert!(state.disabled && !update);
        let (state, update) = state.after(XEmbedMessage::ModalityOff, 0);
        assert!(!state.disabled && !update);
        // A message the mode has no part in.
        assert_eq!(state.after(XEmbedMessage::RequestFocus, 5), (state, false));
    }

    #[test]
    fn a_new_embedder_starts_from_nothing() {
        let state = EmbedState { focused_in_embedder: true, embedder_activated: true, disabled: true, current_embedder: 1 };
        let (state, _) = state.after(XEmbedMessage::EmbeddedNotify, 2);
        assert_eq!(state, EmbedState { current_embedder: 2, ..START });
    }
}
