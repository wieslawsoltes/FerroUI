use super::{IRenderDataVisitor, RenderDataStream};
use crate::media::{BoxShadows, IBrush, IEffect, IPen, RenderOptions, TextOptions};
use crate::platform::{IBitmapImpl, IGeometryImpl, IGlyphRunImpl};
use crate::rendering::scene_graph::{ICustomDrawOperation, LineBoundsHelper};
use crate::{Matrix, Point, Rect, RoundedRect, Thickness};
use std::rc::Rc;
use std::sync::Arc;

/// The bounds accumulated outside of a scope, and how to map the bounds of
/// the scope's content when it ends.
#[derive(Clone, Copy)]
pub struct BoundsScope {
    saved_bounds: Option<Rect>,
    is_transform: bool,
    matrix: Matrix,
    effect_padding: Thickness,
}

/// Computes the bounds of recorded operations.
#[derive(Default)]
pub struct BoundsVisitor {
    pub current: Option<Rect>,
}

impl BoundsVisitor {
    fn enter_child_scope(&mut self, is_transform: bool, matrix: Matrix, effect_padding: Thickness) -> BoundsScope {
        let scope = BoundsScope { saved_bounds: self.current, is_transform, matrix, effect_padding };
        self.current = None;
        scope
    }

    fn plain_scope(&mut self) -> BoundsScope {
        self.enter_child_scope(false, Matrix::default(), Thickness::default())
    }

    fn union(&mut self, rect: Option<Rect>) {
        self.current = Rect::union_optional(self.current, rect);
    }
}

impl IRenderDataVisitor for BoundsVisitor {
    type Scope = BoundsScope;

    fn stop_visiting(&self) -> bool {
        false
    }

    fn on_draw_line(&mut self, server_pen: Option<&dyn IPen>, _client_pen: Option<&dyn IPen>, p1: Point, p2: Point) {
        if let Some(server_pen) = server_pen {
            self.union(Some(LineBoundsHelper::calculate_bounds(p1, p2, server_pen)));
        }
    }

    fn on_draw_rectangle(
        &mut self,
        _server_brush: Option<&dyn IBrush>,
        server_pen: Option<&dyn IPen>,
        _client_pen: Option<&dyn IPen>,
        rect: RoundedRect,
        box_shadows: &BoxShadows,
    ) {
        let bounds = box_shadows.transform_bounds(rect.rect).inflate(server_pen.map_or(0.0, |p| p.thickness()) / 2.0);
        self.union(Some(bounds));
    }

    fn on_draw_ellipse(
        &mut self,
        _server_brush: Option<&dyn IBrush>,
        server_pen: Option<&dyn IPen>,
        _client_pen: Option<&dyn IPen>,
        rect: Rect,
    ) {
        self.union(Some(rect.inflate(server_pen.map_or(0.0, |p| p.thickness()))));
    }

    fn on_draw_geometry(
        &mut self,
        _server_brush: Option<&dyn IBrush>,
        server_pen: Option<&dyn IPen>,
        _client_pen: Option<&dyn IPen>,
        geometry: Option<&Arc<dyn IGeometryImpl>>,
    ) {
        self.union(Some(geometry.map(|g| g.get_render_bounds(server_pen)).unwrap_or_default()));
    }

    fn on_draw_glyph_run(&mut self, _server_brush: Option<&dyn IBrush>, glyph_run: Option<&std::sync::Arc<dyn IGlyphRunImpl>>) {
        self.union(Some(glyph_run.map(|g| g.bounds()).unwrap_or_default()));
    }

    fn on_draw_bitmap(&mut self, _bitmap: Option<&std::sync::Arc<crate::platform::SharedBitmapImpl>>, _opacity: f64, _source_rect: Rect, dest_rect: Rect) {
        self.union(Some(dest_rect));
    }

    fn on_draw_custom(&mut self, operation: Option<&std::sync::Arc<dyn ICustomDrawOperation>>) {
        self.union(operation.map(|o| o.bounds()));
    }

    fn on_push_clip(&mut self, _clip: RoundedRect) -> BoundsScope {
        self.plain_scope()
    }

    fn on_push_geometry_clip(&mut self, _geometry: Option<&Arc<dyn IGeometryImpl>>) -> BoundsScope {
        self.plain_scope()
    }

    fn on_push_opacity(&mut self, _opacity: f64) -> BoundsScope {
        self.plain_scope()
    }

    fn on_push_opacity_mask(&mut self, _brush: Option<&dyn IBrush>, _bounds: Rect) -> BoundsScope {
        self.plain_scope()
    }

    fn on_push_transform(&mut self, matrix: Matrix) -> BoundsScope {
        self.enter_child_scope(true, matrix, Thickness::default())
    }

    fn on_push_render_options(&mut self, _options: RenderOptions) -> BoundsScope {
        self.plain_scope()
    }

    fn on_push_text_options(&mut self, _options: TextOptions) -> BoundsScope {
        self.plain_scope()
    }

    fn on_push_effect(&mut self, effect: Option<&Rc<dyn IEffect>>, _bounds: Rect) -> BoundsScope {
        let padding = crate::media::EffectExtensions::get_effect_output_padding(effect.map(|e| &**e));
        self.enter_child_scope(false, Matrix::default(), padding)
    }

    fn on_pop(&mut self, scope: BoundsScope) {
        let mut child_union = self.current;
        if scope.is_transform {
            child_union = child_union.map(|r| r.transform_to_aabb(scope.matrix));
        } else if scope.effect_padding != Thickness::default() {
            child_union = child_union.map(|r| r.inflate_thickness(scope.effect_padding));
        }
        self.current = Rect::union_optional(scope.saved_bounds, child_union);
    }
}

impl RenderDataStream {
    /// The bounds of everything the stream draws, or `None` if it draws
    /// nothing.
    pub fn calculate_bounds(&self) -> Option<Rect> {
        let mut visitor = BoundsVisitor::default();
        self.visit(&mut visitor);
        visitor.current
    }
}
