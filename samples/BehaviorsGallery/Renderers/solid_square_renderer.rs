//! Port of `Renderers/SolidSquareRenderer.cs` of the reference sample.

use ferroui_base::media::{Brushes, DrawingContext, IBrush};
use ferroui_base::{ferro_markup_type, Rect};
use ferroui_xaml_interactions_custom::IRenderTargetBitmapSimpleRenderer;
use std::rc::Rc;

/// Draws a deep sky blue square.
#[derive(Default)]
pub struct SolidSquareRenderer;

impl PartialEq for SolidSquareRenderer {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl IRenderTargetBitmapSimpleRenderer for SolidSquareRenderer {
    /// `Render(DrawingContext)`.
    fn render(&self, context: &mut DrawingContext<'_>) {
        let brush: Rc<dyn IBrush> = Brushes::deep_sky_blue();
        context.fill_rectangle(&brush, Rect::new(50.0, 50.0, 100.0, 100.0), 0.0);
    }
}

ferro_markup_type!(class SolidSquareRenderer {
    this: Rc<SolidSquareRenderer>,
    handles: [SolidSquareRenderer, Rc<SolidSquareRenderer>, Option<Rc<SolidSquareRenderer>>],
    interfaces: [Rc<dyn IRenderTargetBitmapSimpleRenderer>],
    constructors: [() => || Rc::new(SolidSquareRenderer)],
});
