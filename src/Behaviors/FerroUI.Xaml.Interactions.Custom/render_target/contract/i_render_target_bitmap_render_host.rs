//! Port of `RenderTarget/Contract/IRenderTargetBitmapRenderHost.cs`.

use std::rc::Rc;

/// Defines a render host that can update an underlying
/// [`RenderTargetBitmap`](ferroui_base::media::imaging::RenderTargetBitmap).
pub trait IRenderTargetBitmapRenderHost {
    /// Requests that the render host renders its content to the bitmap.
    fn render(&self);

    /// The identity of the host, used for reference equality: the object behind the handle when
    /// the host is a class of the object model.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }
}

/// Hosts compare by identity (reference equality), so that they can be held in property and
/// untyped values.
impl PartialEq for dyn IRenderTargetBitmapRenderHost {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.reference_id() == other.reference_id()
    }
}

ferroui_base::ferro_markup_type!(interface dyn IRenderTargetBitmapRenderHost as "IRenderTargetBitmapRenderHost" {
    handles: [Rc<dyn IRenderTargetBitmapRenderHost>, Option<Rc<dyn IRenderTargetBitmapRenderHost>>],
});
