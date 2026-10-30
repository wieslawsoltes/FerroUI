//! Records source preview drawing commands while forwarding to the real headless drawing core.
use ferroui_base::media::*;
use ferroui_base::platform::IGeometryImpl;
use ferroui_base::rendering::scene_graph::ICustomDrawOperation;
use ferroui_base::{Matrix, Point, Rect, Ref, RoundedRect};
use std::rc::Rc;
use std::sync::Arc;
pub(super) struct RecordingDrawing {
    inner: PlatformDrawingContext<'static>,
    pub draws: Vec<(Rc<dyn IBrush>, RoundedRect)>,
}
impl RecordingDrawing {
    pub fn new() -> Self {
        Self {
            inner: PlatformDrawingContext::new(
                ferroui_headless::testing::headless_drawing_context_stub(),
            ),
            draws: Vec::new(),
        }
    }
}
impl IDrawingContextCore for RecordingDrawing {
    fn draw_line_core(&mut self, pen: &Rc<dyn IPen>, p1: Point, p2: Point) {
        self.inner.draw_line_core(pen, p1, p2);
    }
    fn draw_geometry_impl_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        geometry: &Arc<dyn IGeometryImpl>,
    ) {
        self.inner.draw_geometry_impl_core(brush, pen, geometry);
    }
    fn draw_rectangle_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        rrect: RoundedRect,
        box_shadows: &BoxShadows,
    ) {
        if let Some(brush) = brush {
            self.draws.push((brush.clone(), rrect));
        }
        self.inner
            .draw_rectangle_core(brush, pen, rrect, box_shadows);
    }
    fn draw_ellipse_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        rect: Rect,
    ) {
        self.inner.draw_ellipse_core(brush, pen, rect);
    }
    fn draw_bitmap(
        &mut self,
        source: &ferroui_base::utilities::RefCounted<ferroui_base::platform::SharedBitmapImpl>,
        opacity: f64,
        source_rect: Rect,
        dest_rect: Rect,
    ) {
        self.inner
            .draw_bitmap(source, opacity, source_rect, dest_rect);
    }
    fn custom(&mut self, custom: &std::sync::Arc<dyn ICustomDrawOperation>) {
        self.inner.custom(custom);
    }
    fn draw_glyph_run(&mut self, foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>) {
        self.inner.draw_glyph_run(foreground, glyph_run);
    }
    fn push_clip_core(&mut self, rect: Rect) {
        self.inner.push_clip_core(rect);
    }
    fn push_rounded_clip_core(&mut self, rect: RoundedRect) {
        self.inner.push_rounded_clip_core(rect);
    }
    fn push_geometry_clip_core(&mut self, clip: &Ref<Geometry>) {
        self.inner.push_geometry_clip_core(clip);
    }
    fn push_opacity_core(&mut self, opacity: f64) {
        self.inner.push_opacity_core(opacity);
    }
    fn push_opacity_mask_core(&mut self, mask: &Rc<dyn IBrush>, bounds: Rect) {
        self.inner.push_opacity_mask_core(mask, bounds);
    }
    fn push_transform_core(&mut self, matrix: Matrix) {
        self.inner.push_transform_core(matrix);
    }
    fn push_render_options_core(&mut self, render_options: RenderOptions) {
        self.inner.push_render_options_core(render_options);
    }
    fn push_text_options_core(&mut self, text_options: TextOptions) {
        self.inner.push_text_options_core(text_options);
    }
    fn push_effect_core(&mut self, effect: &Rc<dyn IEffect>, bounds: Rect) {
        self.inner.push_effect_core(effect, bounds);
    }
    fn pop_clip_core(&mut self) {
        self.inner.pop_clip_core();
    }
    fn pop_geometry_clip_core(&mut self) {
        self.inner.pop_geometry_clip_core();
    }
    fn pop_opacity_core(&mut self) {
        self.inner.pop_opacity_core();
    }
    fn pop_opacity_mask_core(&mut self) {
        self.inner.pop_opacity_mask_core();
    }
    fn pop_transform_core(&mut self) {
        self.inner.pop_transform_core();
    }
    fn pop_render_options_core(&mut self) {
        self.inner.pop_render_options_core();
    }
    fn pop_text_options_core(&mut self) {
        self.inner.pop_text_options_core();
    }
    fn pop_effect_core(&mut self) {
        self.inner.pop_effect_core();
    }
    fn dispose_core(&mut self) {
        self.inner.dispose_core();
    }
}
