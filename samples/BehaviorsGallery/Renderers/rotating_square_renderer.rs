//! Port of `Renderers/RotatingSquareRenderer.cs` of the reference sample.

use ferroui_base::animation::TimeSpan;
use ferroui_base::media::{Brushes, DrawingContext, IBrush};
use ferroui_base::{ferro_markup_type, Matrix, Rect};
use ferroui_xaml_interactions_custom::IRenderTargetBitmapRenderer;
use std::rc::Rc;

/// Draws a fuchsia square that turns around the center of a 200 by 200 target, one radian per
/// second.
#[derive(Default)]
pub struct RotatingSquareRenderer;

impl PartialEq for RotatingSquareRenderer {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl IRenderTargetBitmapRenderer for RotatingSquareRenderer {
    /// `Render(DrawingContext, TimeSpan)`.
    fn render(&self, context: &mut DrawingContext<'_>, elapsed: TimeSpan) {
        let state = context.push_transform(
            Matrix::create_translation(-100.0, -100.0)
                * Matrix::create_rotation(elapsed.total_seconds())
                * Matrix::create_translation(100.0, 100.0),
        );
        let brush: Rc<dyn IBrush> = Brushes::fuchsia();
        context.fill_rectangle(&brush, Rect::new(50.0, 50.0, 100.0, 100.0), 0.0);
        context.pop(state);
    }
}

ferro_markup_type!(class RotatingSquareRenderer {
    this: Rc<RotatingSquareRenderer>,
    handles: [RotatingSquareRenderer, Rc<RotatingSquareRenderer>, Option<Rc<RotatingSquareRenderer>>],
    interfaces: [Rc<dyn IRenderTargetBitmapRenderer>],
    constructors: [() => || Rc::new(RotatingSquareRenderer)],
});
