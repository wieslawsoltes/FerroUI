//! The cursors of the platform (the port of `X11CursorFactory.cs`).

use crate::pixel_buffer::PixelBuffer;
use crate::x11_structs::CursorFontShape;
use crate::xlib::{self, XDisplay, XID};
use ferroui_base::input::StandardCursorType;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::{ICursorFactory, ICursorImpl};
use ferroui_base::PixelPoint;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

const NULL_CURSOR_DATA: [u8; 1] = [0];

/// The shape of the cursor font for a standard cursor (`s_mapping`).
pub(crate) fn font_shape(cursor_type: StandardCursorType) -> Option<CursorFontShape> {
    Some(match cursor_type {
        StandardCursorType::Arrow => CursorFontShape::XC_left_ptr,
        StandardCursorType::Cross => CursorFontShape::XC_cross,
        StandardCursorType::Hand => CursorFontShape::XC_hand2,
        StandardCursorType::Help => CursorFontShape::XC_question_arrow,
        StandardCursorType::Ibeam => CursorFontShape::XC_xterm,
        StandardCursorType::No => CursorFontShape::XC_X_cursor,
        StandardCursorType::Wait => CursorFontShape::XC_watch,
        StandardCursorType::AppStarting => CursorFontShape::XC_watch,
        StandardCursorType::BottomSide => CursorFontShape::XC_bottom_side,
        StandardCursorType::DragCopy => CursorFontShape::XC_center_ptr,
        StandardCursorType::DragLink => CursorFontShape::XC_fleur,
        StandardCursorType::DragMove => CursorFontShape::XC_diamond_cross,
        StandardCursorType::LeftSide => CursorFontShape::XC_left_side,
        StandardCursorType::RightSide => CursorFontShape::XC_right_side,
        StandardCursorType::SizeAll => CursorFontShape::XC_sizing,
        StandardCursorType::TopSide => CursorFontShape::XC_top_side,
        StandardCursorType::UpArrow => CursorFontShape::XC_sb_up_arrow,
        StandardCursorType::BottomLeftCorner => CursorFontShape::XC_bottom_left_corner,
        StandardCursorType::BottomRightCorner => CursorFontShape::XC_bottom_right_corner,
        StandardCursorType::SizeNorthSouth => CursorFontShape::XC_sb_v_double_arrow,
        StandardCursorType::SizeWestEast => CursorFontShape::XC_sb_h_double_arrow,
        StandardCursorType::TopLeftCorner => CursorFontShape::XC_top_left_corner,
        StandardCursorType::TopRightCorner => CursorFontShape::XC_top_right_corner,
        StandardCursorType::None => return None,
    })
}

/// The name of the cursor of the cursor theme for a standard cursor
/// (`s_libraryCursors`).
pub(crate) fn library_cursor(cursor_type: StandardCursorType) -> Option<&'static str> {
    match cursor_type {
        StandardCursorType::DragCopy => Some("dnd-copy"),
        StandardCursorType::DragLink => Some("dnd-link"),
        StandardCursorType::DragMove => Some("dnd-move"),
        // TODO: Check if other platforms have dnd-none, dnd-no-drop and dnd-ask
        _ => None,
    }
}

/// The cursor factory of the X11 platform.
pub struct X11CursorFactory {
    null_cursor: XID,
    display: XDisplay,
    cursors: RefCell<HashMap<StandardCursorType, XID>>,
    drag_no_drop_cursor_handle: Cell<XID>,
}

impl X11CursorFactory {
    pub fn new(display: XDisplay) -> Self {
        Self {
            display,
            null_cursor: Self::get_null_cursor(display),
            cursors: RefCell::new(HashMap::new()),
            drag_no_drop_cursor_handle: Cell::new(0),
        }
    }

    // We don't have a "DragNo" standard cursor type, but Xcursor provides one
    pub fn drag_no_drop_cursor_handle(&self) -> XID {
        if self.drag_no_drop_cursor_handle.get() == 0 {
            self.drag_no_drop_cursor_handle.set(self.load_drag_no_drop_cursor());
        }
        self.drag_no_drop_cursor_handle.get()
    }

    pub fn get_cursor_handle(&self, cursor_type: StandardCursorType) -> XID {
        if cursor_type == StandardCursorType::None {
            self.null_cursor
        } else {
            self.get_cursor_handle_cached(cursor_type)
        }
    }

    fn get_null_cursor(display: XDisplay) -> XID {
        let window = xlib::x_root_window(display, 0);
        let pixmap = xlib::x_create_bitmap_from_data(display, window, &NULL_CURSOR_DATA, 1, 1);
        xlib::x_create_pixmap_cursor(display, pixmap, pixmap, 0, 0)
    }

    fn load_drag_no_drop_cursor(&self) -> XID {
        let mut handle = xlib::xcursor_library_load_cursor(self.display, "dnd-no-drop");

        if handle == 0 {
            handle = self.get_cursor_handle_cached(StandardCursorType::No);
        }
        handle
    }

    fn get_cursor_handle_cached(&self, type_: StandardCursorType) -> XID {
        if let Some(handle) = self.cursors.borrow().get(&type_) {
            return *handle;
        }

        let mut handle: XID = 0;
        if let Some(cursor_name) = library_cursor(type_) {
            handle = xlib::xcursor_library_load_cursor(self.display, cursor_name);
        }

        if handle == 0 {
            if let Some(cursor_shape) = font_shape(type_) {
                handle = xlib::x_create_font_cursor(self.display, cursor_shape as u32);
            }
        }

        if handle == 0 {
            if type_ != StandardCursorType::Arrow {
                handle = self.get_cursor_handle_cached(StandardCursorType::Arrow);
            } else {
                handle = self.null_cursor;
            }
        }

        self.cursors.borrow_mut().insert(type_, handle);

        handle
    }
}

impl ICursorFactory for X11CursorFactory {
    fn get_cursor(&self, cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
        let handle = self.get_cursor_handle(cursor_type);
        Rc::new(CursorImpl::new(handle))
    }

    fn create_cursor(&self, cursor: &Bitmap, hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        Rc::new(CursorImpl::from_image(self.display, cursor, hot_spot))
    }
}

/// A cursor of the server.
///
/// The reference has two classes: `CursorImpl`, a cursor that is shared
/// and never freed, and `XImageCursor`, derived from it, a cursor made of
/// a bitmap that is freed when it is disposed. Both are this type: an
/// image cursor is one that knows the connection to free itself on.
pub struct CursorImpl {
    handle: Cell<XID>,
    image_display: Option<XDisplay>,
}

impl CursorImpl {
    pub fn new(handle: XID) -> Self {
        Self { handle: Cell::new(handle), image_display: None }
    }

    /// `XImageCursor`: a cursor of the pixels of a bitmap.
    fn from_image(display: XDisplay, bitmap: &Bitmap, hot_spot: PixelPoint) -> Self {
        let pixel_size = bitmap.pixel_size();
        let buffer = PixelBuffer::new(pixel_size);
        bitmap.copy_pixels_to_framebuffer(&buffer);
        let mut pixels = buffer.into_pixels();
        let handle = xlib::xcursor_image_load_cursor(
            display,
            pixel_size.width.max(0) as u32,
            pixel_size.height.max(0) as u32,
            hot_spot.x.max(0) as u32,
            hot_spot.y.max(0) as u32,
            &mut pixels,
        );
        Self { handle: Cell::new(handle), image_display: Some(display) }
    }

    pub fn handle(&self) -> XID {
        self.handle.get()
    }

    /// What the handle of an image cursor is (`HandleDescriptor` of
    /// `XImageCursor`).
    pub fn handle_descriptor(&self) -> Option<&'static str> {
        self.image_display.map(|_| "XCURSOR")
    }
}

impl ICursorImpl for CursorImpl {
    fn dispose(&self) {
        if let Some(display) = self.image_display {
            xlib::x_free_cursor(display, self.handle.get());
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn every_standard_cursor_but_none_has_a_font_shape() {
        let all = [
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
        for cursor in all {
            assert!(font_shape(cursor).is_some(), "{cursor:?}");
        }
        assert!(font_shape(StandardCursorType::None).is_none());
    }

    #[test]
    fn the_shapes_are_those_of_the_cursor_font() {
        // The values of X11/cursorfont.h.
        assert_eq!(font_shape(StandardCursorType::Arrow).map(|shape| shape as i32), Some(68));
        assert_eq!(font_shape(StandardCursorType::Ibeam).map(|shape| shape as i32), Some(152));
        assert_eq!(font_shape(StandardCursorType::Hand).map(|shape| shape as i32), Some(60));
        assert_eq!(font_shape(StandardCursorType::Wait), font_shape(StandardCursorType::AppStarting));
        assert_eq!(font_shape(StandardCursorType::No).map(|shape| shape as i32), Some(0));
    }

    #[test]
    fn only_the_drag_cursors_come_from_the_cursor_theme() {
        assert_eq!(library_cursor(StandardCursorType::DragCopy), Some("dnd-copy"));
        assert_eq!(library_cursor(StandardCursorType::DragLink), Some("dnd-link"));
        assert_eq!(library_cursor(StandardCursorType::DragMove), Some("dnd-move"));
        assert_eq!(library_cursor(StandardCursorType::Arrow), None);
    }

    #[test]
    fn a_shared_cursor_is_not_an_image_cursor() {
        let cursor = CursorImpl::new(42);
        assert_eq!(cursor.handle(), 42);
        assert_eq!(cursor.handle_descriptor(), None);
        // Disposing a shared cursor frees nothing (and needs no connection).
        cursor.dispose();
        assert_eq!(cursor.handle(), 42);
    }
}
