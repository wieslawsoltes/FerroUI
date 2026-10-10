//! The state of the root window and of the window manager (the port of
//! `X11Globals.cs`).

use crate::event::Event;
use crate::x11_info::X11Info;
use crate::x11_platform::FerroX11Platform;
use crate::x11_structs::{EventMask, XEventName};
use crate::x11_window_info::X11WindowInfo;
use crate::xlib::{self, Atom, XEvent, XID};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// How the backend learns that a window became active.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WindowActivationTrackingMode {
    #[default]
    FocusEvents,
    _NET_ACTIVE_WINDOW,
    _NET_WM_STATE_FOCUSED,
}

impl WindowActivationTrackingMode {
    /// `Enum.TryParse(text, ignoreCase: true)`.
    pub fn try_parse_ignore_case(text: &str) -> Option<Self> {
        let text = text.trim();
        // The parser of the reference also accepts the numeric value.
        match text {
            "0" => return Some(Self::FocusEvents),
            "1" => return Some(Self::_NET_ACTIVE_WINDOW),
            "2" => return Some(Self::_NET_WM_STATE_FOCUSED),
            _ => {}
        }
        if text.eq_ignore_ascii_case("FocusEvents") {
            Some(Self::FocusEvents)
        } else if text.eq_ignore_ascii_case("_NET_ACTIVE_WINDOW") {
            Some(Self::_NET_ACTIVE_WINDOW)
        } else if text.eq_ignore_ascii_case("_NET_WM_STATE_FOCUSED") {
            Some(Self::_NET_WM_STATE_FOCUSED)
        } else {
            None
        }
    }
}

/// The environment variable that forces an activation tracking mode, for
/// debugging.
pub const FORCE_ACTIVATION_TRACKING_MODE_VARIABLE: &str = "FERROUI_DEBUG_FORCE_X11_ACTIVATION_TRACKING_MODE";

/// Decides how activation is tracked (`GetWindowActivityTrackingMode`):
/// the forced mode, or what the window manager supports.
pub fn get_window_activity_tracking_mode(
    forced_mode: Option<&str>,
    wm: XID,
    supported_features: Option<&[Atom]>,
    net_wm_state_focused: Atom,
    net_active_window: Atom,
) -> WindowActivationTrackingMode {
    if let Some(forced_mode) = forced_mode.and_then(WindowActivationTrackingMode::try_parse_ignore_case) {
        return forced_mode;
    }

    let Some(supported_features) = supported_features else {
        return WindowActivationTrackingMode::FocusEvents;
    };
    if wm == 0 {
        return WindowActivationTrackingMode::FocusEvents;
    }

    if supported_features.contains(&net_wm_state_focused) {
        return WindowActivationTrackingMode::_NET_WM_STATE_FOCUSED;
    }

    if supported_features.contains(&net_active_window) {
        return WindowActivationTrackingMode::_NET_ACTIVE_WINDOW;
    }

    WindowActivationTrackingMode::FocusEvents
}

/// Watches the root window: the window manager, the compositing manager,
/// the root properties and the root geometry.
pub struct X11Globals {
    plat: Weak<FerroX11Platform>,
    screen_number: i32,
    x11: Rc<X11Info>,
    root_window: XID,
    compositing_atom: Atom,

    wm_name: RefCell<Option<String>>,
    composition_atom_owner: Cell<XID>,
    is_composition_enabled: Cell<bool>,
    activation_tracking_mode: Cell<WindowActivationTrackingMode>,
    net_supported: RefCell<Option<Rc<[Atom]>>>,

    pub window_manager_changed: Event,
    pub composition_changed: Event,
    pub root_property_changed: Event<Atom>,
    pub net_active_window_property_changed: Event,
    pub root_geometry_changed_changed: Event,
    pub window_activation_tracking_mode_changed: Event,
    pub net_supported_changed: Event,
}

impl X11Globals {
    pub fn new(plat: &Rc<FerroX11Platform>) -> Rc<Self> {
        let x11 = plat.info();
        let screen_number = xlib::x_default_screen(x11.display());
        let root_window = xlib::x_root_window(x11.display(), screen_number);
        let compositing_atom = xlib::x_intern_atom(x11.display(), &format!("_NET_WM_CM_S{screen_number}"), false);
        let this = Rc::new(Self {
            plat: Rc::downgrade(plat),
            screen_number,
            x11: x11.clone(),
            root_window,
            compositing_atom,
            wm_name: RefCell::new(None),
            composition_atom_owner: Cell::new(0),
            is_composition_enabled: Cell::new(false),
            activation_tracking_mode: Cell::new(WindowActivationTrackingMode::FocusEvents),
            net_supported: RefCell::new(None),
            window_manager_changed: Event::new(),
            composition_changed: Event::new(),
            root_property_changed: Event::new(),
            net_active_window_property_changed: Event::new(),
            root_geometry_changed_changed: Event::new(),
            window_activation_tracking_mode_changed: Event::new(),
            net_supported_changed: Event::new(),
        });

        let weak = Rc::downgrade(&this);
        plat.set_window(
            root_window,
            X11WindowInfo::new(
                Rc::new(move |ev: &mut XEvent| {
                    if let Some(this) = weak.upgrade() {
                        this.on_root_window_event(ev);
                    }
                }),
                None,
            ),
        );
        xlib::x_select_input(
            x11.display(),
            root_window,
            (EventMask::STRUCTURE_NOTIFY_MASK | EventMask::PROPERTY_CHANGE_MASK).bits() as _,
        );

        if x11.has_xkb() {
            xlib::xkb_set_detectable_auto_repeat(x11.display(), true);
        }

        this.on_new_window_manager();
        this.update_compositing_atom_owner();
        this
    }

    /// The number of the screen the globals watch.
    pub fn screen_number(&self) -> i32 {
        self.screen_number
    }

    /// The name of the window manager (`_NET_WM_NAME` of its check
    /// window).
    pub fn wm_name(&self) -> Option<String> {
        self.wm_name.borrow().clone()
    }

    fn set_wm_name(&self, value: Option<String>) {
        if *self.wm_name.borrow() != value {
            *self.wm_name.borrow_mut() = value;
            self.window_manager_changed.raise();
        }
    }

    fn set_composition_atom_owner(&self, value: XID) {
        if self.composition_atom_owner.get() != value {
            self.composition_atom_owner.set(value);
            self.set_is_composition_enabled(value != 0);
        }
    }

    /// Whether a compositing manager runs on the screen.
    pub fn is_composition_enabled(&self) -> bool {
        self.is_composition_enabled.get()
    }

    pub fn set_is_composition_enabled(&self, value: bool) {
        if self.is_composition_enabled.get() != value {
            self.is_composition_enabled.set(value);
            self.composition_changed.raise();
        }
    }

    pub fn activation_tracking_mode(&self) -> WindowActivationTrackingMode {
        self.activation_tracking_mode.get()
    }

    pub fn set_activation_tracking_mode(&self, value: WindowActivationTrackingMode) {
        if self.activation_tracking_mode.get() != value {
            self.activation_tracking_mode.set(value);
            self.window_activation_tracking_mode_changed.raise();
        }
    }

    /// `_NET_SUPPORTED` of the root window, when a window manager runs.
    pub fn net_supported(&self) -> Option<Rc<[Atom]>> {
        self.net_supported.borrow().clone()
    }

    fn set_net_supported(&self, value: Option<Rc<[Atom]>>) {
        *self.net_supported.borrow_mut() = value;
        self.net_supported_changed.raise();
    }

    fn get_supporting_wm_check(&self, _window: XID) -> XID {
        // As the reference, which reads the property of the root window
        // whatever window it is asked about.
        let atoms = self.x11.atoms();
        let property = xlib::x_get_window_property(
            self.x11.display(),
            self.root_window,
            atoms._NET_SUPPORTING_WM_CHECK,
            0,
            std::mem::size_of::<usize>() as _,
            false,
            atoms.WINDOW,
        );
        if property.nitems != 1 {
            return 0;
        }
        if property.actual_type != atoms.WINDOW {
            return 0;
        }
        property.longs().first().copied().unwrap_or(0)
    }

    fn update_compositing_atom_owner(self: &Rc<Self>) {
        // This procedure is described in https://tronche.com/gui/x/icccm/sec-2.html#s-2.8
        let Some(plat) = self.plat.upgrade() else {
            return;
        };
        let display = self.x11.display();

        // Check the server-side selection owner
        let mut new_owner = xlib::x_get_selection_owner(display, self.compositing_atom);
        while self.composition_atom_owner.get() != new_owner {
            // We have a new owner, unsubscribe from the previous one first
            let previous = self.composition_atom_owner.get();
            if previous != 0 {
                plat.remove_window(previous);
                xlib::x_select_input(display, previous, 0);
            }

            // Set it as the current owner and select input
            self.set_composition_atom_owner(new_owner);
            if new_owner != 0 {
                let weak = Rc::downgrade(self);
                plat.set_window(
                    new_owner,
                    X11WindowInfo::new(
                        Rc::new(move |ev: &mut XEvent| {
                            if let Some(this) = weak.upgrade() {
                                this.handle_composition_atom_owner_events(ev);
                            }
                        }),
                        None,
                    ),
                );
                xlib::x_select_input(display, new_owner, EventMask::STRUCTURE_NOTIFY_MASK.bits() as _);
            }

            // Check for the new owner again and repeat the procedure if it was changed between XGetSelectionOwner and XSelectInput call
            new_owner = xlib::x_get_selection_owner(display, self.compositing_atom);
        }
    }

    fn handle_composition_atom_owner_events(self: &Rc<Self>, ev: &mut XEvent) {
        if xlib::event_type(ev) == XEventName::DestroyNotify as i32 {
            self.update_compositing_atom_owner();
        }
    }

    fn get_active_wm(&self) -> XID {
        let wm_window = self.get_supporting_wm_check(self.root_window);
        if wm_window != 0 && wm_window == self.get_supporting_wm_check(wm_window) {
            wm_window
        } else {
            0
        }
    }

    fn get_wm_name(&self, wm: XID) -> Option<String> {
        if wm == 0 {
            return None;
        }
        let atoms = self.x11.atoms();
        let property = xlib::x_get_window_property(
            self.x11.display(),
            wm,
            atoms._NET_WM_NAME,
            0,
            0x7fffffff,
            false,
            atoms.UTF8_STRING,
        );
        if property.nitems == 0 {
            return None;
        }
        if property.actual_format != 8 {
            return None;
        }
        Some(String::from_utf8_lossy(&property.data).into_owned())
    }

    fn on_new_window_manager(&self) {
        let wm = self.get_active_wm();
        let atoms = self.x11.atoms();
        let supported_features: Option<Rc<[Atom]>> = if wm != 0 {
            xlib::x_get_window_property_as_int_ptr_array(
                self.x11.display(),
                self.x11.root_window(),
                atoms._NET_SUPPORTED,
                atoms.ATOM,
            )
            .map(Rc::from)
        } else {
            None
        };
        self.set_wm_name(self.get_wm_name(wm));
        let forced_mode = std::env::var(FORCE_ACTIVATION_TRACKING_MODE_VARIABLE).ok();
        self.set_activation_tracking_mode(get_window_activity_tracking_mode(
            forced_mode.as_deref(),
            wm,
            supported_features.as_deref(),
            atoms._NET_WM_STATE_FOCUSED,
            atoms._NET_ACTIVE_WINDOW,
        ));
        self.set_net_supported(supported_features);
    }

    fn on_root_window_event(self: &Rc<Self>, ev: &mut XEvent) {
        let event_type = xlib::event_type(ev);
        let atoms = self.x11.atoms();
        if event_type == XEventName::PropertyNotify as i32 {
            let atom = xlib::property_event(ev).atom;
            if atom == atoms._NET_SUPPORTING_WM_CHECK {
                self.on_new_window_manager();
            }

            if atom == atoms._NET_ACTIVE_WINDOW {
                self.net_active_window_property_changed.raise();
            }

            self.root_property_changed.invoke(atom);
        }

        if event_type == XEventName::ConfigureNotify as i32 {
            self.root_geometry_changed_changed.raise();
        }

        if event_type == XEventName::ClientMessage as i32 {
            let message = xlib::client_message_event(ev);
            let (message_type, second) = (message.message_type, message.data.get_long(1));
            if message_type == atoms.MANAGER && second as Atom == self.compositing_atom {
                self.update_compositing_atom_owner();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    const FOCUSED: Atom = 301;
    const ACTIVE: Atom = 302;

    fn mode(forced: Option<&str>, wm: XID, supported: Option<&[Atom]>) -> WindowActivationTrackingMode {
        get_window_activity_tracking_mode(forced, wm, supported, FOCUSED, ACTIVE)
    }

    #[test]
    fn without_a_window_manager_activation_follows_focus_events() {
        assert_eq!(mode(None, 0, Some(&[FOCUSED, ACTIVE])), WindowActivationTrackingMode::FocusEvents);
        assert_eq!(mode(None, 7, None), WindowActivationTrackingMode::FocusEvents);
        assert_eq!(mode(None, 7, Some(&[])), WindowActivationTrackingMode::FocusEvents);
    }

    #[test]
    fn the_focused_state_is_preferred_to_the_active_window_property() {
        assert_eq!(mode(None, 7, Some(&[ACTIVE, FOCUSED])), WindowActivationTrackingMode::_NET_WM_STATE_FOCUSED);
        assert_eq!(mode(None, 7, Some(&[ACTIVE])), WindowActivationTrackingMode::_NET_ACTIVE_WINDOW);
    }

    #[test]
    fn a_forced_mode_wins_and_an_unknown_one_is_ignored() {
        assert_eq!(mode(Some("focusevents"), 7, Some(&[FOCUSED])), WindowActivationTrackingMode::FocusEvents);
        assert_eq!(mode(Some("_net_active_window"), 0, None), WindowActivationTrackingMode::_NET_ACTIVE_WINDOW);
        assert_eq!(mode(Some("2"), 0, None), WindowActivationTrackingMode::_NET_WM_STATE_FOCUSED);
        assert_eq!(mode(Some("nonsense"), 7, Some(&[ACTIVE])), WindowActivationTrackingMode::_NET_ACTIVE_WINDOW);
    }
}
