use super::{RenderDataStream, ServerCompositionRenderData};
use crate::media::{IBrush, IImmutableBrush, ISceneBrushContent, ITileBrush, ITransform};
use crate::platform::IDrawingContextImpl;
use crate::{Matrix, Rect, RelativePoint};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// The content of a scene brush recorded without a compositor: a render
/// data stream that refers to the drawn objects directly and is replayed on
/// the thread that recorded it.
pub struct ImmediateRenderDataSceneBrushContent {
    stream: RefCell<Option<RenderDataStream>>,
    brush: Rc<dyn ITileBrush>,
    rect: Rect,
    use_scalable_rasterization: bool,
}

impl ImmediateRenderDataSceneBrushContent {
    /// Creates the content of `brush` from a recorded stream. Without an
    /// explicit `rect` the bounds of the content are the bounds of what the
    /// stream draws, rounded outwards to whole units.
    pub fn new(
        brush: Rc<dyn ITileBrush>,
        stream: RenderDataStream,
        rect: Option<Rect>,
        use_scalable_rasterization: bool,
    ) -> Self {
        let rect = rect
            .or_else(|| ServerCompositionRenderData::apply_render_bounds_rounding_rect(stream.calculate_bounds()))
            .unwrap_or_default();
        Self { stream: RefCell::new(Some(stream)), brush, rect, use_scalable_rasterization }
    }

    fn render_stream(&self, context: &mut dyn IDrawingContextImpl) {
        if let Some(stream) = self.stream.borrow().as_ref() {
            stream.replay(context);
        }
    }

    /// Runs `f` on the recorded stream; `None` once the content has been
    /// disposed.
    #[cfg(test)]
    pub(crate) fn with_stream<R>(&self, f: impl FnOnce(&RenderDataStream) -> R) -> Option<R> {
        self.stream.borrow().as_ref().map(f)
    }
}

impl IBrush for ImmediateRenderDataSceneBrushContent {
    fn opacity(&self) -> f64 {
        self.brush.opacity()
    }

    fn transform(&self) -> Option<Rc<dyn ITransform>> {
        self.brush.transform()
    }

    fn transform_origin(&self) -> RelativePoint {
        self.brush.transform_origin()
    }

    fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        self.brush.relative_transform()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_immutable_brush(self: Rc<Self>) -> Option<Rc<dyn IImmutableBrush>> {
        Some(self)
    }
}

impl IImmutableBrush for ImmediateRenderDataSceneBrushContent {}

impl ISceneBrushContent for ImmediateRenderDataSceneBrushContent {
    fn brush(&self) -> Rc<dyn ITileBrush> {
        self.brush.clone()
    }

    fn rect(&self) -> Rect {
        self.rect
    }

    fn render(&self, context: &mut dyn IDrawingContextImpl, transform: Option<Matrix>) {
        match transform {
            Some(transform) => {
                let old_transform = context.transform();
                context.set_transform(transform * old_transform);
                self.render_stream(context);
                context.set_transform(old_transform);
            }
            None => self.render_stream(context),
        }
    }

    fn use_scalable_rasterization(&self) -> bool {
        self.use_scalable_rasterization
    }

    fn dispose(&self) {
        let Some(mut stream) = self.stream.borrow_mut().take() else { return };
        stream.dispose_resources();
        stream.dispose();
    }
}
