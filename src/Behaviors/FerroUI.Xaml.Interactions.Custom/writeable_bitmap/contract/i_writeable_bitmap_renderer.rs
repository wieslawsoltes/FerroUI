//! Port of `WriteableBitmap/Contract/IWriteableBitmapRenderer.cs`.

use ferroui_base::media::imaging::WriteableBitmap;
use std::rc::Rc;

/// Defines a method used to render into a [`WriteableBitmap`].
pub trait IWriteableBitmapRenderer {
    /// Renders into the provided [`WriteableBitmap`]: `bitmap` is the target bitmap.
    fn render(&self, bitmap: &WriteableBitmap);
}

/// Renderers compare by identity (reference equality), so that they can be held in property and
/// untyped values.
impl PartialEq for dyn IWriteableBitmapRenderer {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

ferroui_base::ferro_markup_type!(interface dyn IWriteableBitmapRenderer as "IWriteableBitmapRenderer" {
    handles: [Rc<dyn IWriteableBitmapRenderer>, Option<Rc<dyn IWriteableBitmapRenderer>>],
});
