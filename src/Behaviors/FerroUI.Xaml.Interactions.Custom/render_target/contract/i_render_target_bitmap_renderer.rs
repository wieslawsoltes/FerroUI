//! Port of `RenderTarget/Contract/IRenderTargetBitmapRenderer.cs`.

use ferroui_base::animation::TimeSpan;
use ferroui_base::media::DrawingContext;
use std::rc::Rc;

/// Provides a method used by [`RenderTargetBitmapBehavior`](crate::RenderTargetBitmapBehavior) to
/// draw onto a render target.
pub trait IRenderTargetBitmapRenderer {
    /// Draws into the provided [`DrawingContext`]: `context` is the drawing context to use,
    /// `elapsed` the elapsed time since the behavior was attached.
    fn render(&self, context: &mut DrawingContext<'_>, elapsed: TimeSpan);
}

/// Renderers compare by identity (reference equality), so that they can be held in property and
/// untyped values.
impl PartialEq for dyn IRenderTargetBitmapRenderer {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

ferroui_base::ferro_markup_type!(interface dyn IRenderTargetBitmapRenderer as "IRenderTargetBitmapRenderer" {
    handles: [Rc<dyn IRenderTargetBitmapRenderer>, Option<Rc<dyn IRenderTargetBitmapRenderer>>],
});
