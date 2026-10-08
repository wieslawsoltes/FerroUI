use crate::media::{BoxShadows, IBrush, IEffect, IPen, RenderOptions, TextOptions};
use crate::platform::{IBitmapImpl, IGeometryImpl, IGlyphRunImpl};
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::{Matrix, Point, Rect, RoundedRect};
use std::rc::Rc;
use std::sync::Arc;

/// Walks the operations of a render data stream. A push returns a scope
/// value that is handed back to `on_pop` when the matching pop is reached.
pub trait IRenderDataVisitor {
    type Scope;

    /// Whether the walk should end early.
    fn stop_visiting(&self) -> bool;

    fn on_draw_line(&mut self, server_pen: Option<&dyn IPen>, client_pen: Option<&dyn IPen>, p1: Point, p2: Point);
    fn on_draw_rectangle(
        &mut self,
        server_brush: Option<&dyn IBrush>,
        server_pen: Option<&dyn IPen>,
        client_pen: Option<&dyn IPen>,
        rect: RoundedRect,
        box_shadows: &BoxShadows,
    );
    fn on_draw_ellipse(
        &mut self,
        server_brush: Option<&dyn IBrush>,
        server_pen: Option<&dyn IPen>,
        client_pen: Option<&dyn IPen>,
        rect: Rect,
    );
    fn on_draw_geometry(
        &mut self,
        server_brush: Option<&dyn IBrush>,
        server_pen: Option<&dyn IPen>,
        client_pen: Option<&dyn IPen>,
        geometry: Option<&Arc<dyn IGeometryImpl>>,
    );
    fn on_draw_glyph_run(&mut self, server_brush: Option<&dyn IBrush>, glyph_run: Option<&std::sync::Arc<dyn IGlyphRunImpl>>);
    fn on_draw_bitmap(&mut self, bitmap: Option<&std::sync::Arc<crate::platform::SharedBitmapImpl>>, opacity: f64, source_rect: Rect, dest_rect: Rect);
    fn on_draw_custom(&mut self, operation: Option<&Rc<dyn ICustomDrawOperation>>);

    fn on_push_clip(&mut self, clip: RoundedRect) -> Self::Scope;
    fn on_push_geometry_clip(&mut self, geometry: Option<&Arc<dyn IGeometryImpl>>) -> Self::Scope;
    fn on_push_opacity(&mut self, opacity: f64) -> Self::Scope;
    fn on_push_opacity_mask(&mut self, brush: Option<&dyn IBrush>, bounds: Rect) -> Self::Scope;
    fn on_push_transform(&mut self, matrix: Matrix) -> Self::Scope;
    fn on_push_render_options(&mut self, options: RenderOptions) -> Self::Scope;
    fn on_push_text_options(&mut self, options: TextOptions) -> Self::Scope;
    fn on_push_effect(&mut self, effect: Option<&Rc<dyn IEffect>>, bounds: Rect) -> Self::Scope;

    fn on_pop(&mut self, scope: Self::Scope);
}
