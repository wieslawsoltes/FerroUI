use super::{draw_with_painted_brushes, IRenderDataVisitor, PaintedShape, RenderDataStream};
use crate::media::{BoxShadows, IBrush, IPen, ImmediateDrawingContext, RenderOptions, TextOptions};
use crate::media::imaging::BitmapBlendingMode;
use crate::platform::{IDrawingContextImpl, IGeometryImpl, IGlyphRunImpl};
use crate::media::effects::ImmutableCustomEffect;
use crate::rendering::composition::effects::{push_custom_effect_graph, pop_custom_effect_graph};
use crate::rendering::scene3d::Scene3DSnapshot;
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::{Matrix, Point, Rect, RoundedRect, Vector};
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
    EffectGraph,
    CompositingLayer,
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
    effect_graphs: Vec<(ImmutableCustomEffect, Rect)>,
    geometry_corner_radius: Option<Vector>,
}

impl<'a> ReplayVisitor<'a> {
    pub fn new(context: &'a mut dyn IDrawingContextImpl) -> Self {
        Self {
            context,
            effect_graphs: Vec::new(),
            geometry_corner_radius: None,
        }
    }
}

impl IRenderDataVisitor for ReplayVisitor<'_> {
    type Scope = ReplayScope;

    fn stop_visiting(&self) -> bool {
        false
    }

    fn on_draw_line(&mut self, server_pen: Option<&dyn IPen>, _client_pen: Option<&dyn IPen>, p1: Point, p2: Point) {
        crate::perf_count!(RenderLinesDrawn);
        if !draw_with_painted_brushes(self.context, None, server_pen, PaintedShape::Line(p1, p2), None) {
            self.context.draw_line(server_pen, p1, p2);
        }
    }

    fn on_draw_rectangle(
        &mut self,
        server_brush: Option<&dyn IBrush>,
        server_pen: Option<&dyn IPen>,
        _client_pen: Option<&dyn IPen>,
        rect: RoundedRect,
        box_shadows: &BoxShadows,
    ) {
        crate::perf_count!(RenderRectanglesDrawn);
        // A brush the compositor paints itself (winui-materials.md).
        if !draw_with_painted_brushes(self.context, server_brush, server_pen, PaintedShape::Rectangle(rect), Some(box_shadows)) {
            self.context.draw_rectangle(server_brush, server_pen, rect, box_shadows);
        }
    }

    fn on_draw_ellipse(
        &mut self,
        server_brush: Option<&dyn IBrush>,
        server_pen: Option<&dyn IPen>,
        _client_pen: Option<&dyn IPen>,
        rect: Rect,
    ) {
        crate::perf_count!(RenderEllipsesDrawn);
        if !draw_with_painted_brushes(self.context, server_brush, server_pen, PaintedShape::Ellipse(rect), None) {
            self.context.draw_ellipse(server_brush, server_pen, rect);
        }
    }

    fn on_draw_geometry(
        &mut self,
        server_brush: Option<&dyn IBrush>,
        server_pen: Option<&dyn IPen>,
        _client_pen: Option<&dyn IPen>,
        geometry: Option<&Arc<dyn IGeometryImpl>>,
    ) {
        if let Some(geometry) = geometry {
            if let Some(radius) = self.geometry_corner_radius {
                let bounds = geometry.bounds();
                let rect = RoundedRect::from_radius_xy(
                    bounds, radius.x.min(bounds.width / 2.0), radius.y.min(bounds.height / 2.0),
                );
                self.on_draw_rectangle(server_brush, server_pen, _client_pen, rect, &BoxShadows::default());
                return;
            }
            crate::perf_count!(RenderGeometriesDrawn);
            if !draw_with_painted_brushes(self.context, server_brush, server_pen, PaintedShape::Geometry(&**geometry), None) {
                self.context.draw_geometry(server_brush, server_pen, &**geometry);
            }
        }
    }

    fn on_draw_glyph_run(&mut self, server_brush: Option<&dyn IBrush>, glyph_run: Option<&std::sync::Arc<dyn IGlyphRunImpl>>) {
        crate::perf_count!(RenderGlyphRunsDrawn);
        if let Some(glyph_run) = glyph_run {
            if !draw_with_painted_brushes(self.context, server_brush, None, PaintedShape::GlyphRun(&**glyph_run), None) {
                self.context.draw_glyph_run(server_brush, &**glyph_run);
            }
        }
    }

    fn on_draw_bitmap(&mut self, bitmap: Option<&std::sync::Arc<crate::platform::SharedBitmapImpl>>, opacity: f64, source_rect: Rect, dest_rect: Rect) {
        crate::perf_count!(RenderBitmapsDrawn);
        if let Some(bitmap) = bitmap {
            self.context.draw_bitmap(&**bitmap, opacity, source_rect, dest_rect);
        }
    }

    fn on_draw_custom(&mut self, operation: Option<&std::sync::Arc<dyn ICustomDrawOperation>>) {
        if let Some(operation) = operation {
            let mut context = ImmediateDrawingContext::borrowed(self.context);
            operation.render(&mut context);
            context.dispose();
        }
    }

    fn on_draw_scene_3d(&mut self, scene: Option<&Arc<Scene3DSnapshot>>, bounds: Rect) {
        // Drawn by the 3D capability of the context; without one nothing is
        // drawn, and the scene is counted and reported (#608).
        if let Some(scene) = scene {
            crate::rendering::scene3d::draw_scene_3d(self.context, scene, bounds);
        }
    }

    fn on_push_clip(&mut self, clip: RoundedRect) -> ReplayScope {
        crate::perf_count!(RenderClipsPushed);
        self.context.push_clip_rounded(clip);
        ReplayScope::new(ReplayScopeKind::Clip, true)
    }

    fn on_push_geometry_clip(&mut self, geometry: Option<&Arc<dyn IGeometryImpl>>) -> ReplayScope {
        crate::perf_count!(RenderGeometryClipsPushed);
        if let Some(geometry) = geometry {
            self.context.push_geometry_clip(&**geometry);
        }
        ReplayScope::new(ReplayScopeKind::GeometryClip, geometry.is_some())
    }

    fn on_push_opacity(&mut self, opacity: f64) -> ReplayScope {
        crate::perf_count!(RenderLayersPushed);
        if opacity != 1.0 {
            self.context.push_opacity(opacity, None);
        }
        ReplayScope::new(ReplayScopeKind::Opacity, opacity != 1.0)
    }

    fn on_push_compositing_layer(&mut self, blend: BitmapBlendingMode, opacity: f64, bounds: Rect) -> ReplayScope {
        crate::perf_count!(RenderLayersPushed);
        // Recording accepts this opcode only for a backend with compositing layers. A stream
        // replayed into a context without them is drawn with the opacity of the layer and
        // without its blend, as a frame is never left unpainted.
        if self.context.try_push_compositing_layer(blend, opacity, bounds) {
            return ReplayScope::new(ReplayScopeKind::CompositingLayer, true);
        }
        self.on_push_opacity(opacity)
    }

    fn on_push_opacity_mask(&mut self, brush: Option<&dyn IBrush>, bounds: Rect) -> ReplayScope {
        crate::perf_count!(RenderLayersPushed);
        if let Some(brush) = brush {
            self.context.push_opacity_mask(brush, bounds);
        }
        ReplayScope::new(ReplayScopeKind::OpacityMask, brush.is_some())
    }

    fn on_push_transform(&mut self, matrix: Matrix) -> ReplayScope {
        crate::perf_count!(RenderTransformsPushed);
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

    fn on_push_effect(&mut self, effect: Option<&std::sync::Arc<dyn crate::media::IImmutableEffect>>, bounds: Rect) -> ReplayScope {
        crate::perf_count!(RenderLayersPushed);
        let mut active = false;
        if let Some(effect) = effect {
            if let Some(custom) = effect.as_custom_effect() {
                if push_custom_effect_graph(self.context, custom, bounds) {
                    self.effect_graphs.push((custom.clone(), bounds));
                    return ReplayScope::new(ReplayScopeKind::EffectGraph, true);
                }
            }
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
            ReplayScopeKind::CompositingLayer => self.context.pop_compositing_layer(),
            ReplayScopeKind::OpacityMask => self.context.pop_opacity_mask(),
            ReplayScopeKind::Transform => self.context.set_transform(scope.saved_transform),
            ReplayScopeKind::RenderOptions => self.context.pop_render_options(),
            ReplayScopeKind::TextOptions => self.context.pop_text_options(),
            ReplayScopeKind::EffectGraph => {
                let (effect, bounds) = self.effect_graphs.pop().expect("Unbalanced custom effect graph");
                pop_custom_effect_graph(self.context, &effect, bounds);
            }
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

    /// Replays rectangular shape geometry with render-side corner radii. Resources and
    /// scopes still use the normal replay path, including live and composition brushes.
    pub fn replay_as_rounded_rectangle(&self, context: &mut dyn IDrawingContextImpl, scopes: &mut Vec<ReplayScope>, radius: Vector) {
        let mut visitor = ReplayVisitor::new(context);
        visitor.geometry_corner_radius = Some(radius);
        self.visit_with_scopes(&mut visitor, scopes);
    }
}
