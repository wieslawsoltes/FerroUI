//! Whether a window is the active one (the port of
//! `ActivityTrackingHelper.cs`).

use crate::event::Event;
use crate::x11_globals::WindowActivationTrackingMode;
use crate::x11_platform::FerroX11Platform;
use crate::x11_structs::{NotifyMode, XEventName};
use crate::x11_window::X11Window;
use crate::xlib::{self, Atom, XEvent, XID};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use std::cell::Cell;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// What the helper needs from a focus event to decide
/// (`WindowActivationTrackingHelper.OnEvent`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FocusDecision {
    /// The event says nothing about activation.
    Ignore,
    /// The window became active or inactive.
    SetActive(bool),
}

/// Decides what a focus event means for activation in a tracking mode
/// (after the attempt to activate a transient child, which comes first).
pub(crate) fn decide_focus_event(mode: WindowActivationTrackingMode, focus_in: bool, notify_mode: i32) -> FocusDecision {
    if mode != WindowActivationTrackingMode::FocusEvents {
        return FocusDecision::Ignore;
    }

    // See: https://github.com/fltk/fltk/issues/295
    if notify_mode != NotifyMode::NotifyNormal as i32 {
        return FocusDecision::Ignore;
    }

    FocusDecision::SetActive(focus_in)
}

/// Tracks the activation of one window, by the mode the window manager
/// allows.
pub struct WindowActivationTrackingHelper {
    platform: Weak<FerroX11Platform>,
    window: Weak<X11Window>,
    window_handle: XID,
    active: Cell<bool>,

    // Set when the current active state came from a speculative transfer (see SetActiveSpeculatively).
    // Only meaningful in _NET_WM_STATE_FOCUSED mode, where the guess is re-verified against the real
    // _NET_WM_STATE the next time the active window changes.
    speculative: Cell<bool>,

    pub activation_changed: Event<bool>,
    subscriptions: Cell<(u64, u64)>,
}

impl WindowActivationTrackingHelper {
    pub fn new(platform: &Rc<FerroX11Platform>, window: &Weak<X11Window>, window_handle: XID) -> Rc<Self> {
        let this = Rc::new(Self {
            platform: Rc::downgrade(platform),
            window: window.clone(),
            window_handle,
            active: Cell::new(false),
            speculative: Cell::new(false),
            activation_changed: Event::new(),
            subscriptions: Cell::new((0, 0)),
        });
        let weak = Rc::downgrade(&this);
        let active_window = platform.active_window_tracker().active_window_changed.subscribe({
            let weak = weak.clone();
            move |active_window| {
                if let Some(this) = weak.upgrade() {
                    this.on_active_window_changed(active_window);
                }
            }
        });
        let mode = platform.globals().window_activation_tracking_mode_changed.subscribe(move |()| {
            if let Some(this) = weak.upgrade() {
                this.on_window_activation_tracking_mode_changed();
            }
        });
        this.subscriptions.set((active_window, mode));
        this
    }

    pub fn is_active(&self) -> bool {
        self.active.get()
    }

    fn set_active(&self, active: bool) {
        if active != self.active.get() {
            self.active.set(active);
            self.activation_changed.invoke(active);
        }
    }

    fn mode(&self) -> WindowActivationTrackingMode {
        self.platform
            .upgrade()
            .map_or(WindowActivationTrackingMode::FocusEvents, |platform| platform.globals().activation_tracking_mode())
    }

    fn requery_net_wm_state(&self) {
        let Some(platform) = self.platform.upgrade() else {
            return;
        };
        let atoms = platform.info().atoms();
        let state = xlib::x_get_window_property_as_int_ptr_array(
            platform.display(),
            self.window_handle,
            atoms._NET_WM_STATE,
            atoms.ATOM,
        )
        .unwrap_or_default();
        self.on_net_wm_state_changed(&state);
    }

    fn requery_activation(&self) {
        // The _NET_ACTIVE_WINDOW mode is driven by the shared X11ActiveWindowTracker, so only the
        // per-window _NET_WM_STATE needs re-reading here.
        if self.mode() == WindowActivationTrackingMode::_NET_WM_STATE_FOCUSED {
            self.requery_net_wm_state();
        }
    }

    fn on_window_activation_tracking_mode_changed(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        DispatcherTimer::run_once(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.requery_activation();
                }
            },
            Duration::from_secs(1),
            DispatcherPriority::INPUT,
        );
    }

    pub fn on_event(&self, ev: &XEvent) {
        let event_type = xlib::event_type(ev);
        let focus_in = event_type == XEventName::FocusIn as i32;
        if !focus_in && event_type != XEventName::FocusOut as i32 {
            return;
        }

        // Always attempt to activate transient children on focus events
        if focus_in && self.window.upgrade().is_some_and(|window| window.activate_transient_child_if_needed()) {
            return;
        }

        if let FocusDecision::SetActive(active) =
            decide_focus_event(self.mode(), focus_in, xlib::focus_change_event(ev).mode)
        {
            self.set_active(active);
        }
    }

    fn on_active_window_changed(&self, active_window: XID) {
        let mode = self.mode();
        if mode == WindowActivationTrackingMode::_NET_ACTIVE_WINDOW {
            // Authoritative in this mode — the published XID is the active window.
            self.speculative.set(false);
            self.set_active(active_window == self.window_handle);
        }
        // In _NET_WM_STATE_FOCUSED mode the root _NET_ACTIVE_WINDOW change is only a "focus moved"
        // pulse; use it to re-verify a speculative activation against the authoritative _NET_WM_STATE.
        else if mode == WindowActivationTrackingMode::_NET_WM_STATE_FOCUSED && self.speculative.get() {
            self.requery_net_wm_state();
        }
    }

    pub fn set_active_speculatively(&self) {
        self.speculative.set(true);
        self.set_active(true);
    }

    pub fn dispose(&self) {
        let Some(platform) = self.platform.upgrade() else {
            return;
        };
        let (active_window, mode) = self.subscriptions.get();
        platform.active_window_tracker().active_window_changed.unsubscribe(active_window);
        platform.globals().window_activation_tracking_mode_changed.unsubscribe(mode);
    }

    pub fn on_net_wm_state_changed(&self, atoms: &[Atom]) {
        if self.mode() == WindowActivationTrackingMode::_NET_WM_STATE_FOCUSED {
            let Some(platform) = self.platform.upgrade() else {
                return;
            };
            // Authoritative signal — it overrides any speculative guess.
            self.speculative.set(false);
            self.set_active(atoms.contains(&platform.info().atoms()._NET_WM_STATE_FOCUSED));
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn focus_events_decide_only_in_their_mode_and_only_normal_ones() {
        let normal = NotifyMode::NotifyNormal as i32;
        let grab = NotifyMode::NotifyGrab as i32;
        assert_eq!(decide_focus_event(WindowActivationTrackingMode::FocusEvents, true, normal), FocusDecision::SetActive(true));
        assert_eq!(decide_focus_event(WindowActivationTrackingMode::FocusEvents, false, normal), FocusDecision::SetActive(false));
        // A grab (a menu of the window manager, a key binding) moves the focus without a change of the active window.
        assert_eq!(decide_focus_event(WindowActivationTrackingMode::FocusEvents, false, grab), FocusDecision::Ignore);
        assert_eq!(decide_focus_event(WindowActivationTrackingMode::_NET_ACTIVE_WINDOW, true, normal), FocusDecision::Ignore);
        assert_eq!(decide_focus_event(WindowActivationTrackingMode::_NET_WM_STATE_FOCUSED, true, normal), FocusDecision::Ignore);
    }
}
