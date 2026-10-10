//! The progress of a compose sequence (the port of `XkbComposeState.cs`).

use crate::xkb_compose_table::XkbComposeTable;
use xkbcommon_dl::{xkb_compose_state, xkb_compose_state_flags, xkb_compose_status, xkbcommon_compose_option};

/// What a key symbol did to a compose sequence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XkbComposeStatus {
    /// Not in a sequence.
    Nothing,
    /// In the middle of a sequence.
    Composing,
    /// The sequence is complete.
    Composed,
    /// The sequence was cancelled by the symbol.
    Cancelled,
}

/// Managed wrapper around `xkb_compose_state*`.
/// Tracks compose/dead-key sequence state on the Wayland thread.
pub struct XkbComposeState {
    state: *mut xkb_compose_state,
}

impl XkbComposeState {
    pub fn new(table: &XkbComposeTable) -> Option<Self> {
        let xkb = xkbcommon_compose_option()?;
        // SAFETY: the table is alive; the state takes a reference of its own to it.
        let state = unsafe { (xkb.xkb_compose_state_new)(table.handle(), xkb_compose_state_flags::XKB_COMPOSE_STATE_NO_FLAGS) };
        if state.is_null() {
            None
        } else {
            Some(Self { state })
        }
    }

    /// Feeds a keysym into the compose state machine.
    /// Returns the compose status after processing.
    pub fn feed(&self, keysym: u32) -> XkbComposeStatus {
        let Some(xkb) = xkbcommon_compose_option() else {
            return XkbComposeStatus::Nothing;
        };
        // SAFETY: the state is alive; the symbol is a value.
        let status = unsafe {
            (xkb.xkb_compose_state_feed)(self.state, keysym);
            (xkb.xkb_compose_state_get_status)(self.state)
        };
        match status {
            xkb_compose_status::XKB_COMPOSE_NOTHING => XkbComposeStatus::Nothing,
            xkb_compose_status::XKB_COMPOSE_COMPOSING => XkbComposeStatus::Composing,
            xkb_compose_status::XKB_COMPOSE_COMPOSED => XkbComposeStatus::Composed,
            xkb_compose_status::XKB_COMPOSE_CANCELLED => XkbComposeStatus::Cancelled,
        }
    }

    /// Gets the composed UTF-8 text after a successful compose sequence.
    pub fn get_composed_text(&self) -> Option<String> {
        let xkb = xkbcommon_compose_option()?;
        // SAFETY: a null buffer of size zero asks for the length only.
        let needed = unsafe { (xkb.xkb_compose_state_get_utf8)(self.state, std::ptr::null_mut(), 0) };
        if needed <= 0 {
            return None;
        }
        let mut buffer = vec![0u8; needed as usize + 1];
        // SAFETY: the buffer has the size given, which the function does not write beyond.
        unsafe { (xkb.xkb_compose_state_get_utf8)(self.state, buffer.as_mut_ptr().cast(), buffer.len()) };
        buffer.truncate(needed as usize);
        Some(String::from_utf8_lossy(&buffer).into_owned())
    }

    /// Gets the composed keysym after a successful compose sequence.
    pub fn get_composed_keysym(&self) -> u32 {
        match xkbcommon_compose_option() {
            // SAFETY: the state is alive.
            Some(xkb) => unsafe { (xkb.xkb_compose_state_get_one_sym)(self.state) },
            None => 0,
        }
    }

    /// Resets compose state (e.g., on focus loss).
    pub fn reset(&self) {
        if let Some(xkb) = xkbcommon_compose_option() {
            // SAFETY: the state is alive.
            unsafe { (xkb.xkb_compose_state_reset)(self.state) };
        }
    }
}

impl Drop for XkbComposeState {
    fn drop(&mut self) {
        if let Some(xkb) = xkbcommon_compose_option() {
            // SAFETY: the one reference this value owns, given up once.
            unsafe { (xkb.xkb_compose_state_unref)(self.state) };
        }
    }
}
