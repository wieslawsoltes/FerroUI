use crate::platform::IDrawingContextImpl;

/// What a visual renders its own content with.
pub struct ServerVisualRenderContext<'a> {
    canvas: &'a mut dyn IDrawingContextImpl,
}

impl<'a> ServerVisualRenderContext<'a> {
    pub fn new(canvas: &'a mut dyn IDrawingContextImpl) -> Self {
        Self { canvas }
    }

    pub fn canvas(&mut self) -> &mut dyn IDrawingContextImpl {
        self.canvas
    }
}
