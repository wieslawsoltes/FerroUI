//! The themed cursors of a connection (the port of
//! `WaylandCursorManager.cs`).
//!
//! The reference loads the theme with `libwayland-cursor`. The port reads
//! the same theme files with the crate `wayland-cursor`, under the name and
//! the size the reference passes (the default theme, 24).

use crate::server::persistent::w_surface::WlSurfaceData;
use crate::server::wayland_worker::WaylandWorkerState;
use crate::wayland_exception::FerroWaylandException;
use ferroui_base::input::StandardCursorType;
use std::collections::HashMap;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, QueueHandle};
use wayland_cursor::CursorTheme;

/// A themed cursor: a surface with the first image of the cursor attached.
pub struct CursorEntry {
    pub surface: WlSurface,
    pub hotspot_x: i32,
    pub hotspot_y: i32,
}

/// The names a standard cursor is looked up by in a theme, in order.
pub fn cursor_names(cursor_type: StandardCursorType) -> &'static [&'static str] {
    match cursor_type {
        StandardCursorType::Arrow => &["default", "left_ptr"],
        StandardCursorType::Ibeam => &["text", "xterm"],
        StandardCursorType::Wait => &["wait", "watch"],
        StandardCursorType::Cross => &["crosshair", "cross"],
        StandardCursorType::UpArrow => &["up_arrow", "sb_up_arrow"],
        StandardCursorType::SizeWestEast => &["ew-resize", "col-resize", "sb_h_double_arrow"],
        StandardCursorType::SizeNorthSouth => &["ns-resize", "row-resize", "sb_v_double_arrow"],
        StandardCursorType::SizeAll => &["all-scroll", "fleur"],
        StandardCursorType::No => &["not-allowed", "crossed_circle"],
        StandardCursorType::Hand => &["pointer", "hand2", "pointing_hand"],
        StandardCursorType::AppStarting => &["progress", "left_ptr_watch"],
        StandardCursorType::Help => &["help", "question_arrow"],
        StandardCursorType::TopSide => &["top_side", "n-resize"],
        StandardCursorType::BottomSide => &["bottom_side", "s-resize"],
        StandardCursorType::LeftSide => &["left_side", "w-resize"],
        StandardCursorType::RightSide => &["right_side", "e-resize"],
        StandardCursorType::TopLeftCorner => &["top_left_corner", "nw-resize"],
        StandardCursorType::TopRightCorner => &["top_right_corner", "ne-resize"],
        StandardCursorType::BottomLeftCorner => &["bottom_left_corner", "sw-resize"],
        StandardCursorType::BottomRightCorner => &["bottom_right_corner", "se-resize"],
        StandardCursorType::DragMove => &["grabbing", "dnd-move"],
        StandardCursorType::DragCopy => &["copy", "dnd-copy"],
        StandardCursorType::DragLink => &["alias", "dnd-link"],
        _ => &[],
    }
}

/// The standard cursors that have names.
const CURSOR_TYPES: [StandardCursorType; 23] = [
    StandardCursorType::Arrow,
    StandardCursorType::Ibeam,
    StandardCursorType::Wait,
    StandardCursorType::Cross,
    StandardCursorType::UpArrow,
    StandardCursorType::SizeWestEast,
    StandardCursorType::SizeNorthSouth,
    StandardCursorType::SizeAll,
    StandardCursorType::No,
    StandardCursorType::Hand,
    StandardCursorType::AppStarting,
    StandardCursorType::Help,
    StandardCursorType::TopSide,
    StandardCursorType::BottomSide,
    StandardCursorType::LeftSide,
    StandardCursorType::RightSide,
    StandardCursorType::TopLeftCorner,
    StandardCursorType::TopRightCorner,
    StandardCursorType::BottomLeftCorner,
    StandardCursorType::BottomRightCorner,
    StandardCursorType::DragMove,
    StandardCursorType::DragCopy,
    StandardCursorType::DragLink,
];

pub struct WaylandCursorManager {
    // The theme owns the buffers of the images, which the surfaces have attached.
    _theme: CursorTheme,
    cursors: HashMap<StandardCursorType, Option<CursorEntry>>,
}

impl WaylandCursorManager {
    pub fn new(
        display: &Connection,
        shm: &WlShm,
        compositor: &WlCompositor,
        queue_handle: &QueueHandle<WaylandWorkerState>,
    ) -> Result<Self, FerroWaylandException> {
        let mut theme = CursorTheme::load_from_name(display, shm.clone(), "default", 24)
            .map_err(|_| FerroWaylandException::new("Failed to load default cursor theme"))?;
        let mut cursors = HashMap::new();
        for cursor_type in CURSOR_TYPES {
            cursors.insert(cursor_type, Self::load_cursor(&mut theme, compositor, queue_handle, cursor_type));
        }
        Ok(Self { _theme: theme, cursors })
    }

    fn load_cursor(
        theme: &mut CursorTheme,
        compositor: &WlCompositor,
        queue_handle: &QueueHandle<WaylandWorkerState>,
        cursor_type: StandardCursorType,
    ) -> Option<CursorEntry> {
        let name = cursor_names(cursor_type).iter().find(|name| theme.get_cursor(name).is_some())?;
        let cursor = theme.get_cursor(name)?;
        if cursor.image_count() == 0 {
            return None;
        }

        let image = &cursor[0];
        let (hotspot_x, hotspot_y) = image.hotspot();
        let surface = compositor.create_surface(queue_handle, WlSurfaceData::Cursor);
        surface.attach(Some(image), 0, 0);
        surface.commit();
        Some(CursorEntry { surface, hotspot_x: hotspot_x as i32, hotspot_y: hotspot_y as i32 })
    }

    /// The cursor of a type: `None` for the type that hides the cursor; the arrow for a type
    /// the theme does not have.
    pub fn get_cursor(&self, cursor_type: StandardCursorType) -> Option<&CursorEntry> {
        if cursor_type == StandardCursorType::None {
            return None;
        }
        match self.cursors.get(&cursor_type) {
            Some(Some(entry)) => Some(entry),
            Some(None) if cursor_type == StandardCursorType::Arrow => None,
            _ => self.get_cursor(StandardCursorType::Arrow),
        }
    }

    pub fn dispose(&mut self) {
        for (_, entry) in self.cursors.drain() {
            if let Some(entry) = entry {
                entry.surface.destroy();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn every_named_cursor_has_names_and_the_hidden_one_has_none() {
        for cursor_type in CURSOR_TYPES {
            assert!(!cursor_names(cursor_type).is_empty(), "{cursor_type:?}");
        }
        assert_eq!(cursor_names(StandardCursorType::Arrow), ["default", "left_ptr"]);
        assert_eq!(cursor_names(StandardCursorType::Hand), ["pointer", "hand2", "pointing_hand"]);
        assert!(cursor_names(StandardCursorType::None).is_empty());
    }
}
