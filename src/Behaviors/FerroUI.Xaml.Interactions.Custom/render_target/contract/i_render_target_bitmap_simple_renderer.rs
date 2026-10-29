//! Port of `RenderTarget/Contract/IRenderTargetBitmapSimpleRenderer.cs`.

use ferroui_base::media::DrawingContext;
use std::rc::Rc;

/// Provides a method used by
/// [`StaticRenderTargetBitmapBehavior`](crate::StaticRenderTargetBitmapBehavior) to draw onto a
/// render target.
pub trait IRenderTargetBitmapSimpleRenderer {
    /// Draws into the provided [`DrawingContext`]: `context` is the drawing context to use.
    fn render(&self, context: &mut DrawingContext<'_>);
}

/// Renderers compare by identity (reference equality), so that they can be held in property and
/// untyped values.
impl PartialEq for dyn IRenderTargetBitmapSimpleRenderer {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

ferroui_base::ferro_markup_type!(interface dyn IRenderTargetBitmapSimpleRenderer as "IRenderTargetBitmapSimpleRenderer" {
    handles: [Rc<dyn IRenderTargetBitmapSimpleRenderer>, Option<Rc<dyn IRenderTargetBitmapSimpleRenderer>>],
});
