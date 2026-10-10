//! The cookie of an input event (the port of `WaylandInputEventCookie.cs`).

use std::sync::{Mutex, PoisonError};
use wayland_client::protocol::wl_seat::WlSeat;

/// Opaque platform-specific cookie attached to raw input events.
/// Carries the Wayland seat and serial needed for requests like xdg_toplevel.move/resize.
/// Must NOT be accessed from the UI thread — only passed back to the worker.
///
/// The cookie remembers the connection it was created on by its number, so that stale
/// cookies from a previous connection are rejected at consume time.
///
/// `PostOob` of the reference (a call on the worker with the globals the cookie came from)
/// belongs to the drag source: stage 2 of `docs/porting/wayland-platform.md`.
pub struct WaylandInputEventCookie {
    seat: Mutex<Option<WlSeat>>,
    serial: u32,
    connection_id: u64,
}

impl WaylandInputEventCookie {
    pub fn new(seat: WlSeat, serial: u32, connection_id: u64) -> Self {
        Self { seat: Mutex::new(Some(seat)), serial, connection_id }
    }

    /// Consumes the cookie, returning the seat and serial. Can only be consumed once.
    /// Returns `None` if already consumed or if the cookie belongs to a different
    /// connection (e.g. after a reconnect).
    pub fn try_consume(&self, current_connection_id: u64) -> Option<(WlSeat, u32)> {
        let seat = self.seat.lock().unwrap_or_else(PoisonError::into_inner).take()?;
        if self.connection_id == current_connection_id {
            Some((seat, self.serial))
        } else {
            None
        }
    }
}
