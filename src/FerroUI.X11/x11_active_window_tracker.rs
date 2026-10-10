//! The active window as the window manager publishes it (the port of
//! `X11ActiveWindowTracker.cs`).

use crate::event::Event;
use crate::x11_platform::FerroX11Platform;
use crate::xlib::{self, XID};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use std::cell::Cell;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// Follows `_NET_ACTIVE_WINDOW` of the root window.
pub struct X11ActiveWindowTracker {
    platform: Weak<FerroX11Platform>,
    active_window: Cell<XID>,

    pub active_window_changed: Event<XID>,
}

impl X11ActiveWindowTracker {
    pub fn new(platform: &Rc<FerroX11Platform>) -> Rc<Self> {
        let this = Rc::new(Self {
            platform: Rc::downgrade(platform),
            active_window: Cell::new(0),
            active_window_changed: Event::new(),
        });
        let globals = platform.globals();
        let weak = Rc::downgrade(&this);
        globals.net_active_window_property_changed.subscribe({
            let weak = weak.clone();
            move |()| {
                if let Some(this) = weak.upgrade() {
                    this.requery();
                }
            }
        });
        globals.window_activation_tracking_mode_changed.subscribe({
            let weak = weak.clone();
            move |()| {
                if let Some(this) = weak.upgrade() {
                    this.on_mode_changed();
                }
            }
        });
        globals.net_supported_changed.subscribe(move |()| {
            if let Some(this) = weak.upgrade() {
                this.requery();
            }
        });
        this.requery();
        this
    }

    // Whether the WM publishes root _NET_ACTIVE_WINDOW at all. Speculative activations can only be
    // auto-corrected when it does, since that's the notification we re-verify against.
    pub fn tracks_root_active_window(&self) -> bool {
        let Some(platform) = self.platform.upgrade() else {
            return false;
        };
        let active_window = platform.info().atoms()._NET_ACTIVE_WINDOW;
        platform.globals().net_supported().is_some_and(|supported| supported.contains(&active_window))
    }

    fn set_active_window(&self, window: XID) {
        if self.active_window.get() != window {
            self.active_window.set(window);
            self.active_window_changed.invoke(window);
        }
    }

    fn requery(&self) {
        if !self.tracks_root_active_window() {
            return;
        }
        let Some(platform) = self.platform.upgrade() else {
            return;
        };
        let info = platform.info();
        let value = xlib::x_get_window_property_as_int_ptr_array(
            info.display(),
            info.root_window(),
            info.atoms()._NET_ACTIVE_WINDOW,
            info.atoms().WINDOW,
        );
        self.set_active_window(value.and_then(|value| value.first().copied()).unwrap_or(0));
    }

    fn on_mode_changed(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        DispatcherTimer::run_once(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.requery();
                }
            },
            Duration::from_secs(1),
            DispatcherPriority::INPUT,
        );
    }
}
