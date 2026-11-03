//! Native keyboard information for nested compositors and input bridges.
use std::sync::Arc;

/// Raw XKB masks and active layout group from this seat's last modifiers event.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct WaylandKeyModifiers {
    pub depressed: u32,
    pub latched: u32,
    pub locked: u32,
    pub group: u32,
}

/// Snapshot associated with a routed key when native keyboard capture is enabled.
/// Each event shares the compiled seat map until the compositor replaces it.
#[derive(Debug, Clone)]
pub struct WaylandKeyEvent {
    /// Original unshifted wl_keyboard keycode (evdev, without the XKB +8).
    pub code: u32,
    /// Validated and expanded XKB text-v1 map.
    pub keymap: Option<Arc<str>>,
    pub modifiers: WaylandKeyModifiers,
    /// Locally generated key repeat, as distinct from a physical press.
    pub repeat: bool,
}
