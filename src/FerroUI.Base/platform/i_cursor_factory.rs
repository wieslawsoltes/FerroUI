use super::ICursorImpl;
use crate::input::StandardCursorType;
use crate::media::imaging::Bitmap;
use crate::PixelPoint;
use std::rc::Rc;

/// Creates the platform implementations of mouse cursors.
///
/// This is an implementation detail of the platform backends.
pub trait ICursorFactory {
    /// Gets the platform cursor for a standard cursor type.
    fn get_cursor(&self, cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl>;

    /// Creates a platform cursor from a bitmap. `hot_spot` is the position
    /// within the bitmap that is the position of the pointer.
    ///
    /// Every backend is expected to implement this. The default exists so
    /// that a backend written before bitmap cursors were part of the
    /// contract keeps building.
    ///
    /// # Panics
    /// The default implementation panics: the backend does not support
    /// bitmap cursors.
    fn create_cursor(&self, cursor: &Bitmap, hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        let _ = (cursor, hot_spot);
        panic!("The cursor factory of the platform does not support bitmap cursors.")
    }
}
