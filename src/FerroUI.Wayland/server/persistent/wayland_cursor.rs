//! The worker's cursors (the port of `WaylandCursor.cs`).

use super::wayland_bitmap_cursor::WaylandBitmapCursor;
use crate::server::transient::wayland_cursor_manager::WaylandCursorManager;
use ferroui_base::input::StandardCursorType;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use wayland_client::protocol::wl_surface::WlSurface;

/// The number of a cursor of the worker: what the UI thread holds of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WaylandCursorId(u64);

impl WaylandCursorId {
    /// A number no other cursor has.
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for WaylandCursorId {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolved, connection-bound cursor image: the surface to attach and its hotspot.
#[derive(Clone, Debug)]
pub struct WaylandCursorImage {
    pub surface: WlSurface,
    pub hotspot_x: i32,
    pub hotspot_y: i32,
}

/// Worker-side representation of a pointer cursor. The UI thread only ever holds the
/// proxy of it; this object lives on the wayland thread.
///
/// The reference has an abstract class with two derived ones; here they are the cases of one
/// type.
pub enum WaylandCursor {
    /// A themed cursor. Stateless and connection-independent — resolved against the current
    /// cursor manager at use, so it survives reconnects for free.
    Standard(WaylandStandardCursor),
    /// A cursor of the pixels of a bitmap.
    Bitmap(WaylandBitmapCursor),
}

/// The cursors of the worker by their numbers.
pub type WaylandCursors = HashMap<WaylandCursorId, WaylandCursor>;

impl WaylandCursor {
    /// Resolves the cursor against the cursor theme of the current connection. Returns `None` if the
    /// cursor has no image right now (e.g. a disconnected bitmap cursor, or a hidden one).
    pub fn resolve(&self, cursor_manager: &WaylandCursorManager) -> Option<WaylandCursorImage> {
        match self {
            WaylandCursor::Standard(cursor) => cursor.resolve(cursor_manager),
            WaylandCursor::Bitmap(cursor) => cursor.resolve(),
        }
    }
}

/// A themed cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaylandStandardCursor {
    cursor_type: StandardCursorType,
}

impl WaylandStandardCursor {
    pub fn new(cursor_type: StandardCursorType) -> Self {
        Self { cursor_type }
    }

    pub fn resolve(&self, cursor_manager: &WaylandCursorManager) -> Option<WaylandCursorImage> {
        let entry = cursor_manager.get_cursor(self.cursor_type)?;
        Some(WaylandCursorImage { surface: entry.surface.clone(), hotspot_x: entry.hotspot_x, hotspot_y: entry.hotspot_y })
    }
}

/// The image of the cursor of a surface: the cursor it asked for, or the default arrow when it
/// asked for none (or for one that is gone).
pub fn resolve_cursor(
    cursor: Option<WaylandCursorId>,
    cursors: &WaylandCursors,
    cursor_manager: &WaylandCursorManager,
) -> Option<WaylandCursorImage> {
    // Shared, stateless fallback used when a surface hasn't requested a specific cursor.
    const DEFAULT_CURSOR: WaylandStandardCursor = WaylandStandardCursor { cursor_type: StandardCursorType::Arrow };
    match cursor.and_then(|cursor| cursors.get(&cursor)) {
        Some(cursor) => cursor.resolve(cursor_manager),
        None => DEFAULT_CURSOR.resolve(cursor_manager),
    }
}
