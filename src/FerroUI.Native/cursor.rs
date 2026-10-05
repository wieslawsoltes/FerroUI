//! Mouse cursors.

use crate::helpers::{to_frn_cursor_type, ComResultExt};
use crate::interop::*;
use ferroui_base::input::StandardCursorType;
use ferroui_base::media::imaging::{Bitmap, BitmapEncoderOptions, PngBitmapEncoderOptions};
use ferroui_base::platform::{ICursorFactory, ICursorImpl};
use ferroui_base::PixelPoint;
use ferroui_microcom::ComPtr;
use std::any::Any;
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;

/// A native cursor.
pub struct FerroNativeCursor {
    cursor: RefCell<Option<ComPtr<IFrnCursor>>>,
}

impl FerroNativeCursor {
    pub fn new(cursor: Option<ComPtr<IFrnCursor>>) -> Rc<FerroNativeCursor> {
        Rc::new(FerroNativeCursor { cursor: RefCell::new(cursor) })
    }

    /// The native cursor; `None` once disposed.
    pub fn cursor(&self) -> Option<ComPtr<IFrnCursor>> {
        self.cursor.borrow().clone()
    }

    pub fn handle(&self) -> isize {
        0
    }

    pub fn handle_descriptor(&self) -> &'static str {
        "<none>"
    }

    /// The cursor behind a cursor implementation handle, if it is a cursor
    /// of this backend.
    pub(crate) fn from_cursor_impl(cursor: &dyn ICursorImpl) -> Option<&FerroNativeCursor> {
        cursor.as_any().downcast_ref::<FerroNativeCursor>()
    }
}

impl ICursorImpl for FerroNativeCursor {
    fn dispose(&self) {
        let cursor = self.cursor.borrow_mut().take();
        drop(cursor);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Creates the cursors of the macOS backend.
pub struct CursorFactory {
    native: ComPtr<IFrnCursorFactory>,
}

impl CursorFactory {
    pub fn new(native: ComPtr<IFrnCursorFactory>) -> Self {
        Self { native }
    }

    /// Creates a cursor from an encoded image (PNG); `hot_spot` is the
    /// pixel of the image that is the pointer position.
    fn create_cursor_from_image_data(&self, mut image_data: Vec<u8>, hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        // SAFETY: the native side copies the image data during the call.
        let native_cursor = unsafe {
            self.native.create_custom_cursor(
                image_data.as_mut_ptr() as *mut c_void,
                image_data.len(),
                FrnPixelSize { width: hot_spot.x, height: hot_spot.y },
            )
        }
        .check();

        FerroNativeCursor::new(native_cursor)
    }
}

impl ICursorFactory for CursorFactory {
    fn get_cursor(&self, cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
        let cursor = self.native.get_cursor(to_frn_cursor_type(cursor_type)).check();
        FerroNativeCursor::new(cursor)
    }

    /// # Panics
    /// Panics when the bitmap cannot be encoded, where the reference
    /// implementation throws.
    fn create_cursor(&self, cursor: &Bitmap, hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        let mut image_data = Vec::new();
        if let Err(error) = cursor.save(&mut image_data, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT))
        {
            panic!("Unable to save the cursor bitmap: {error}");
        }

        self.create_cursor_from_image_data(image_data, hot_spot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct OtherCursor;
    impl ICursorImpl for OtherCursor {
        fn dispose(&self) {}
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn cursors_of_this_backend_are_recovered_from_their_contract_handle() {
        let cursor = FerroNativeCursor::new(None);
        let handle: Rc<dyn ICursorImpl> = cursor.clone();
        let found = FerroNativeCursor::from_cursor_impl(&*handle).expect("a cursor of this backend");
        assert!(std::ptr::eq(found, &*cursor));
        assert_eq!(cursor.handle(), 0);
        assert_eq!(cursor.handle_descriptor(), "<none>");

        let other: Rc<dyn ICursorImpl> = Rc::new(OtherCursor);
        assert!(FerroNativeCursor::from_cursor_impl(&*other).is_none());
    }
}
