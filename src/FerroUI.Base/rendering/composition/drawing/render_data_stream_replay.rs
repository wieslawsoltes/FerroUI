use super::{IRenderDataVisitor, RenderDataStream};
use crate::media::{BoxShadows, IBrush, IEffect, IPen, ImmediateDrawingContext, RenderOptions, TextOptions};
use crate::platform::{IBitmapImpl, IDrawingContextImpl, IGeometryImpl, IGlyphRunImpl};
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::{Matrix, Point, Rect, RoundedRect};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReplayScopeKind {
    Clip,
    GeometryClip,
    Opacity,
    OpacityMask,
    Transform,
    RenderOptions,
    TextOptions,
    Effect,
}

/// What a replayed push has to undo.
#[derive(Clone, Copy)]
pub struct ReplayScope {
    kind: ReplayScopeKind,
    active: bool,
    saved_transform: Matrix,
}

impl ReplayScope {
    fn new(kind: ReplayScopeKind, active: bool) -> Self {
        Self { kind, active, saved_transform: Matrix::IDENTITY }
    }
}

/// Replays recorded operations onto a platform drawing context.
pub struct ReplayVisitor<'a> {
    context: &'a mut dyn IDrawingContextImpl,
}

impl<'a> ReplayVisitor<'a> {
    pub fn new(context: &'a mut dyn IDrawingContextImpl) -> Self {
        Self { context }
    }
}

impl IRenderDataVisitor for ReplayVisitor<'_> {
    type Scope = ReplayScope;

    fn stop_visiting(&self) -> bool {
        false
    }

    fn on_draw_line(&mut self, server_pen: Option<&dyn IPen>, _client_pen: Option<&dyn IPen>, p1: Point, p2: Point) {
        self.context.draw_line(server_pen, p1, p2);
    }

    fn on_draw_rectangle(
        &mut self,
        server_brush: Option<&dyn IBrush>,
        server_pen: Option<&dyn IPen>,
        _client_pen: Option<&dyn IPen>,
        rect: RoundedRect,
        box_shadows: &BoxShadows,
    ) {
        self.context.draw_rectangle(server_brush, server_pen, rect, box_shadows);
    }

    fn on_draw_ellipse(
        &mut self,
        server_brush: Option<&dyn IBrush>,
        server_pen: Option<&dyn IPen>,
        _client_pen: Option<&dyn IPen>,
        rect: Rect,
    ) {
        self.context.draw_ellipse(server_brush, server_pen, rect);
    }

    fn on_draw_geometry(
        &mut self,
        server_brush: Option<&dyn IBrush>,
        server_pen: Option<&dyn IPen>,
        _client_pen: Option<&dyn IPen>,
        geometry: Option<&Arc<dyn IGeometryImpl>>,
    ) {
        if let Some(geometry) = geometry {
            self.context.draw_geometry(server_brush, server_pen, &**geometry);
        }
    }

    fn on_draw_glyph_run(&mut self, server_brush: Option<&dyn IBrush>, glyph_run: Option<&std::sync::Arc<dyn IGlyphRunImpl>>) {
        if let Some(glyph_run) = glyph_run {
            self.context.draw_glyph_run(server_brush, &**glyph_run);
        }
    }

    fn on_draw_bitmap(&mut self, bitmap: Option<&Rc<dyn IBitmapImpl>>, opacity: f64, source_rect: Rect, dest_rect: Rect) {
        if let Some(bitmap) = bitmap {
            self.context.draw_bitmap(&**bitmap, opacity, source_rect, dest_rect);
        }
    }

    fn on_draw_custom(&mut self, operation: Option<&Rc<dyn ICustomDrawOperation>>) {
        if let Some(operation) = operation {
            let mut context = ImmediateDrawingContext::borrowed(self.context);
            operation.render(&mut context);
            context.dispose();
        }
    }

    fn on_push_clip(&mut self, clip: RoundedRect) -> ReplayScope {
        self.context.push_clip_rounded(clip);
        ReplayScope::new(ReplayScopeKind::Clip, true)
    }

    fn on_push_geometry_clip(&mut self, geometry: Option<&Arc<dyn IGeometryImpl>>) -> ReplayScope {
        if let Some(geometry) = geometry {
            self.context.push_geometry_clip(&**geometry);
        }
        ReplayScope::new(ReplayScopeKind::GeometryClip, geometry.is_some())
    }

    fn on_push_opacity(&mut self, opacity: f64) -> ReplayScope {
        if opacity != 1.0 {
            self.context.push_opacity(opacity, None);
        }
        ReplayScope::new(ReplayScopeKind::Opacity, opacity != 1.0)
    }

    fn on_push_opacity_mask(&mut self, brush: Option<&dyn IBrush>, bounds: Rect) -> ReplayScope {
        if let Some(brush) = brush {
            self.context.push_opacity_mask(brush, bounds);
        }
        ReplayScope::new(ReplayScopeKind::OpacityMask, brush.is_some())
    }

    fn on_push_transform(&mut self, matrix: Matrix) -> ReplayScope {
        let saved = self.context.transform();
        self.context.set_transform(matrix * saved);
        ReplayScope { kind: ReplayScopeKind::Transform, active: true, saved_transform: saved }
    }

    fn on_push_render_options(&mut self, options: RenderOptions) -> ReplayScope {
        self.context.push_render_options(options);
        ReplayScope::new(ReplayScopeKind::RenderOptions, true)
    }

    fn on_push_text_options(&mut self, options: TextOptions) -> ReplayScope {
        self.context.push_text_options(options);
        ReplayScope::new(ReplayScopeKind::TextOptions, true)
    }

    fn on_push_effect(&mut self, effect: Option<&Rc<dyn IEffect>>, bounds: Rect) -> ReplayScope {
        let mut active = false;
        if let Some(effect) = effect {
            if let Some(effect_impl) = self.context.as_drawing_context_impl_with_effects() {
                effect_impl.push_effect(Some(bounds), &**effect);
                active = true;
            }
        }
        ReplayScope::new(ReplayScopeKind::Effect, active)
    }

    fn on_pop(&mut self, scope: ReplayScope) {
        if !scope.active {
            return;
        }
        match scope.kind {
            ReplayScopeKind::Clip => self.context.pop_clip(),
            ReplayScopeKind::GeometryClip => self.context.pop_geometry_clip(),
            ReplayScopeKind::Opacity => self.context.pop_opacity(),
            ReplayScopeKind::OpacityMask => self.context.pop_opacity_mask(),
            ReplayScopeKind::Transform => self.context.set_transform(scope.saved_transform),
            ReplayScopeKind::RenderOptions => self.context.pop_render_options(),
            ReplayScopeKind::TextOptions => self.context.pop_text_options(),
            ReplayScopeKind::Effect => {
                if let Some(effect_impl) = self.context.as_drawing_context_impl_with_effects() {
                    effect_impl.pop_effect();
                }
            }
        }
    }
}

impl RenderDataStream {
    /// Replays the recorded operations onto a platform drawing context.
    pub fn replay(&self, context: &mut dyn IDrawingContextImpl) {
        let mut visitor = ReplayVisitor::new(context);
        self.visit(&mut visitor);
    }

    /// Like [`replay`](Self::replay), with a reusable scope stack.
    pub fn replay_with_scopes(&self, context: &mut dyn IDrawingContextImpl, scopes: &mut Vec<ReplayScope>) {
        let mut visitor = ReplayVisitor::new(context);
        self.visit_with_scopes(&mut visitor, scopes);
    }
}
