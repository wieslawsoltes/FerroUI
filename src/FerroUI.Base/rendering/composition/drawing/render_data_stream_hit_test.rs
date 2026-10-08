use super::{IRenderDataVisitor, RenderDataStream};
use crate::media::{
    BoxShadows, CombinedGeometry, EllipseGeometry, Geometry, IBrush, IEffect, IPen, ImmutableGeometry,
    IntersectionResult, LineGeometry, MatrixTransform, RectangleGeometry, RenderOptions, TextOptions,
};
use crate::platform::{IBitmapImpl, IGeometryImpl, IGlyphRunImpl};
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::{Matrix, Point, Rect, Ref, RoundedRect};
use std::rc::Rc;
use std::sync::Arc;

/// What a hit-tested push has to restore.
pub struct HitTestScope {
    saved_live: bool,
    restore_point: bool,
    saved_point: Option<Point>,
}

impl HitTestScope {
    fn new(saved_live: bool) -> Self {
        Self { saved_live, restore_point: false, saved_point: None }
    }
}

/// Hit tests recorded operations against a point or a geometry.
pub struct HitTestVisitor {
    stop_visiting: bool,
    pub hit_found: bool,
    pub hit_result: IntersectionResult,
    current_geometry: Option<Ref<Geometry>>,
    pub current_point: Option<Point>,
    pub live: bool,
    saved_geometries: Option<Vec<Ref<Geometry>>>,
}

impl HitTestVisitor {
    fn new() -> Self {
        Self {
            stop_visiting: false,
            hit_found: false,
            hit_result: IntersectionResult::NotCalculated,
            current_geometry: None,
            current_point: None,
            live: true,
            saved_geometries: None,
        }
    }

    pub fn for_geometry(geometry: Ref<Geometry>) -> Self {
        Self { current_geometry: Some(geometry), saved_geometries: Some(Vec::new()), ..Self::new() }
    }

    pub fn for_point(point: Point) -> Self {
        Self { current_point: Some(point), ..Self::new() }
    }

    fn hit(&mut self) {
        self.hit_found = true;
        self.stop_visiting = true;
    }

    fn hit_with(&mut self, result: IntersectionResult) {
        self.hit_result = result;
        self.stop_visiting = true;
    }

    fn get_intersection_detail(first_rect: Rect, second_rect: Rect) -> IntersectionResult {
        if first_rect.contains_rect(second_rect) {
            return IntersectionResult::FullyContains;
        }
        // Mirrors upstream, which compares the second rectangle with itself
        // here.
        if second_rect.contains_rect(second_rect) {
            return IntersectionResult::FullyInside;
        }
        if first_rect.intersects(second_rect) {
            return IntersectionResult::Intersects;
        }
        IntersectionResult::Empty
    }
}

impl IRenderDataVisitor for HitTestVisitor {
    type Scope = HitTestScope;

    fn stop_visiting(&self) -> bool {
        self.stop_visiting
    }

    fn on_draw_line(&mut self, _server_pen: Option<&dyn IPen>, client_pen: Option<&dyn IPen>, p1: Point, p2: Point) {
        if !self.live {
            return;
        }
        if let Some(point) = self.current_point {
            if hit_test_line(client_pen, p1, p2, point) {
                self.hit();
                return;
            }
        }
        if let Some(geometry) = self.current_geometry.clone() {
            self.hit_with(hit_test_line_geometry(client_pen, p1, p2, &geometry));
        }
    }

    fn on_draw_rectangle(
        &mut self,
        server_brush: Option<&dyn IBrush>,
        _server_pen: Option<&dyn IPen>,
        client_pen: Option<&dyn IPen>,
        rect: RoundedRect,
        _box_shadows: &BoxShadows,
    ) {
        if !self.live {
            return;
        }
        if let Some(point) = self.current_point {
            if hit_test_rectangle(server_brush, client_pen, rect, point) {
                self.hit();
                return;
            }
        }
        if let Some(geometry) = self.current_geometry.clone() {
            self.hit_with(hit_test_rectangle_geometry(server_brush, client_pen, rect, &geometry));
        }
    }

    fn on_draw_ellipse(
        &mut self,
        server_brush: Option<&dyn IBrush>,
        _server_pen: Option<&dyn IPen>,
        client_pen: Option<&dyn IPen>,
        rect: Rect,
    ) {
        if !self.live {
            return;
        }
        if let Some(point) = self.current_point {
            if hit_test_ellipse(server_brush, client_pen, rect, point) {
                self.hit();
                return;
            }
        }
        if let Some(geometry) = self.current_geometry.clone() {
            self.hit_with(hit_test_ellipse_geometry(server_brush, client_pen, rect, &geometry));
        }
    }

    fn on_draw_geometry(
        &mut self,
        server_brush: Option<&dyn IBrush>,
        _server_pen: Option<&dyn IPen>,
        client_pen: Option<&dyn IPen>,
        geometry: Option<&Arc<dyn IGeometryImpl>>,
    ) {
        let Some(geometry) = geometry else { return };
        if !self.live {
            return;
        }
        if let Some(point) = self.current_point {
            if (server_brush.is_some() && geometry.fill_contains(point))
                || (client_pen.is_some() && geometry.stroke_contains(client_pen, point))
            {
                self.hit();
                return;
            }
        }
        if let Some(current_geometry_impl) = self.current_geometry.as_ref().and_then(|g| g.platform_impl()) {
            let result = hit_test_geometry(server_brush, client_pen, Some(&current_geometry_impl), Some(geometry));
            if (result as i32) > (IntersectionResult::Empty as i32) {
                self.hit_with(result);
            }
        }
    }

    fn on_draw_glyph_run(&mut self, _server_brush: Option<&dyn IBrush>, glyph_run: Option<&std::sync::Arc<dyn IGlyphRunImpl>>) {
        let Some(glyph_run) = glyph_run else { return };
        if !self.live {
            return;
        }
        if let Some(point) = self.current_point {
            if glyph_run.bounds().contains_exclusive(point) {
                self.hit();
                return;
            }
        }
        if let Some(geometry) = &self.current_geometry {
            let result = Self::get_intersection_detail(glyph_run.bounds(), geometry.bounds());
            self.hit_with(result);
        }
    }

    fn on_draw_bitmap(&mut self, _bitmap: Option<&std::sync::Arc<crate::platform::SharedBitmapImpl>>, _opacity: f64, _source_rect: Rect, dest_rect: Rect) {
        if !self.live {
            return;
        }
        if let Some(point) = self.current_point {
            if dest_rect.contains(point) {
                self.hit();
                return;
            }
        }
        if let Some(geometry) = &self.current_geometry {
            let result = Self::get_intersection_detail(dest_rect, geometry.bounds());
            self.hit_with(result);
        }
    }

    fn on_draw_custom(&mut self, operation: Option<&Rc<dyn ICustomDrawOperation>>) {
        if !self.live {
            return;
        }
        let Some(operation) = operation else { return };
        if let Some(point) = self.current_point {
            if operation.hit_test(point) {
                self.hit();
                return;
            }
        }
        if let Some(geometry) = self.current_geometry.clone() {
            self.hit_with(operation.hit_test_geometry(&geometry));
        }
    }

    fn on_push_clip(&mut self, clip: RoundedRect) -> HitTestScope {
        let scope = HitTestScope::new(self.live);
        if self.live
            && (self.current_point.is_some_and(|point| !clip.rect.contains(point))
                || self.current_geometry.as_ref().is_some_and(|geometry| !clip.rect.contains_rect(geometry.bounds())))
        {
            self.live = false;
        }
        scope
    }

    fn on_push_geometry_clip(&mut self, geometry: Option<&Arc<dyn IGeometryImpl>>) -> HitTestScope {
        let scope = HitTestScope::new(self.live);
        if let (true, Some(geometry)) = (self.live, geometry) {
            if self.current_point.is_some_and(|point| !geometry.fill_contains(point))
                || self.current_geometry.as_ref().and_then(|g| g.platform_impl()).is_some_and(|current| {
                    current.get_fill_intersection_result(&**geometry) == IntersectionResult::Empty
                })
            {
                self.live = false;
            }
        }
        scope
    }

    fn on_push_opacity(&mut self, _opacity: f64) -> HitTestScope {
        HitTestScope::new(self.live)
    }

    fn on_push_opacity_mask(&mut self, _brush: Option<&dyn IBrush>, _bounds: Rect) -> HitTestScope {
        HitTestScope::new(self.live)
    }

    fn on_push_transform(&mut self, matrix: Matrix) -> HitTestScope {
        let mut scope = HitTestScope::new(self.live);
        if self.live {
            match matrix.try_invert() {
                Some(inverted) => {
                    scope.restore_point = true;
                    if let Some(point) = self.current_point {
                        scope.saved_point = Some(point);
                        self.current_point = Some(inverted.transform(point));
                    } else if let Some(current) = self.current_geometry.take() {
                        if let Some(saved) = self.saved_geometries.as_mut() {
                            saved.push(current.clone());
                        }
                        let clone = current.clone_geometry();
                        let existing = clone.transform().map_or(Matrix::IDENTITY, |t| t.value());
                        clone.set_transform(MatrixTransform::with_matrix(existing * inverted));
                        self.current_geometry = Some(clone);
                    }
                }
                None => self.live = false,
            }
        }
        scope
    }

    fn on_push_render_options(&mut self, _options: RenderOptions) -> HitTestScope {
        HitTestScope::new(self.live)
    }

    fn on_push_text_options(&mut self, _options: TextOptions) -> HitTestScope {
        HitTestScope::new(self.live)
    }

    fn on_push_effect(&mut self, _effect: Option<&Rc<dyn IEffect>>, _bounds: Rect) -> HitTestScope {
        HitTestScope::new(self.live)
    }

    fn on_pop(&mut self, scope: HitTestScope) {
        self.live = scope.saved_live;
        if scope.restore_point {
            self.current_point = scope.saved_point;
            self.current_geometry = self.saved_geometries.as_mut().and_then(Vec::pop);
        }
    }
}

impl RenderDataStream {
    /// Whether anything recorded is hit at a point.
    pub fn hit_test(&self, point: Point) -> bool {
        let mut visitor = HitTestVisitor::for_point(point);
        self.visit(&mut visitor);
        visitor.hit_found
    }

    /// How the recorded content intersects a geometry.
    pub fn hit_test_geometry(&self, geometry: &Ref<Geometry>) -> IntersectionResult {
        let mut visitor = HitTestVisitor::for_geometry(geometry.clone());
        self.visit(&mut visitor);
        visitor.hit_result
    }
}

fn hit_test_line(client_pen: Option<&dyn IPen>, p1: Point, p2: Point, p: Point) -> bool {
    let Some(client_pen) = client_pen else { return false };

    let half_thickness = client_pen.thickness() / 2.0;
    let min_x = p1.x.min(p2.x) - half_thickness;
    let max_x = p1.x.max(p2.x) + half_thickness;
    let min_y = p1.y.min(p2.y) - half_thickness;
    let max_y = p1.y.max(p2.y) + half_thickness;

    if p.x < min_x || p.x > max_x || p.y < min_y || p.y > max_y {
        return false;
    }

    let length = |x: f64, y: f64| (x * x + y * y).sqrt();

    let (ap_x, ap_y) = (p.x - p1.x, p.y - p1.y);
    let dot1 = (p2.x - p1.x) * ap_x + (p2.y - p1.y) * ap_y;
    if dot1 < 0.0 {
        return length(ap_x, ap_y) <= half_thickness;
    }

    let (bp_x, bp_y) = (p.x - p2.x, p.y - p2.y);
    let dot2 = (p1.x - p2.x) * bp_x + (p1.y - p2.y) * bp_y;
    if dot2 < 0.0 {
        return length(bp_x, bp_y) <= half_thickness;
    }

    let b_xa_x = p2.x - p1.x;
    let b_ya_y = p2.y - p1.y;

    let distance = (b_xa_x * (p.y - p1.y) - b_ya_y * (p.x - p1.x)) / (b_xa_x * b_xa_x + b_ya_y * b_ya_y).sqrt();

    distance.abs() <= half_thickness
}

fn hit_test_line_geometry(client_pen: Option<&dyn IPen>, p1: Point, p2: Point, geometry: &Ref<Geometry>) -> IntersectionResult {
    if client_pen.is_none() {
        return IntersectionResult::NotCalculated;
    }
    hit_test_geometry(
        None,
        client_pen,
        geometry.platform_impl().as_ref(),
        LineGeometry::with_points(p1, p2).platform_impl().as_ref(),
    )
}

fn hit_test_rectangle(server_brush: Option<&dyn IBrush>, client_pen: Option<&dyn IPen>, rect: RoundedRect, p: Point) -> bool {
    let stroke_thickness_adjustment = client_pen.map_or(0.0, |pen| pen.thickness() / 2.0);

    if rect.is_rounded() {
        let outer = rect.inflate(stroke_thickness_adjustment, stroke_thickness_adjustment);
        if outer.contains_exclusive(p) {
            if server_brush.is_some() {
                return true;
            }
            let inner = rect.deflate(stroke_thickness_adjustment, stroke_thickness_adjustment);
            return !inner.contains_exclusive(p);
        }
    } else {
        let outer = rect.rect.inflate(stroke_thickness_adjustment);
        if outer.contains_exclusive(p) {
            if server_brush.is_some() {
                return true;
            }
            let inner = rect.rect.deflate(stroke_thickness_adjustment);
            return !inner.contains_exclusive(p);
        }
    }

    false
}

fn hit_test_rectangle_geometry(
    server_brush: Option<&dyn IBrush>,
    client_pen: Option<&dyn IPen>,
    rect: RoundedRect,
    geometry: &Ref<Geometry>,
) -> IntersectionResult {
    let target = if rect.is_rounded() {
        RectangleGeometry::with_rect_and_radii(rect.rect, rect.radii_top_left.x, rect.radii_top_left.y)
    } else {
        RectangleGeometry::with_rect(rect.rect)
    };
    hit_test_geometry(server_brush, client_pen, geometry.platform_impl().as_ref(), target.platform_impl().as_ref())
}

fn hit_test_geometry(
    server_brush: Option<&dyn IBrush>,
    client_pen: Option<&dyn IPen>,
    current_geometry: Option<&Arc<dyn IGeometryImpl>>,
    target_geometry: Option<&Arc<dyn IGeometryImpl>>,
) -> IntersectionResult {
    let (Some(current_geometry), Some(target_geometry)) = (current_geometry, target_geometry) else {
        return IntersectionResult::Empty;
    };

    match (server_brush, client_pen) {
        (None, Some(client_pen)) => {
            current_geometry.get_fill_intersection_result(&*target_geometry.get_widened_geometry(client_pen))
        }
        (Some(_), Some(client_pen)) if client_pen.thickness() > 0.0 => {
            let stroke_geometry = target_geometry.get_widened_geometry(client_pen);
            let combined = CombinedGeometry::with_geometries(
                ImmutableGeometry::new(Some(stroke_geometry)).upcast(),
                ImmutableGeometry::new(Some(target_geometry.clone())).upcast(),
            );
            match combined.platform_impl() {
                Some(combined_impl) => current_geometry.get_fill_intersection_result(&*combined_impl),
                None => IntersectionResult::Empty,
            }
        }
        (Some(_), _) => current_geometry.get_fill_intersection_result(&**target_geometry),
        (None, None) => IntersectionResult::Empty,
    }
}

fn hit_test_ellipse(server_brush: Option<&dyn IBrush>, client_pen: Option<&dyn IPen>, rect: Rect, p: Point) -> bool {
    let center = rect.center();

    let stroke_thickness = client_pen.map_or(0.0, |pen| pen.thickness());

    let mut rx = rect.width / 2.0 + stroke_thickness / 2.0;
    let mut ry = rect.height / 2.0 + stroke_thickness / 2.0;

    let dx = p.x - center.x;
    let dy = p.y - center.y;

    if dx.abs() > rx || dy.abs() > ry {
        return false;
    }

    if server_brush.is_some() {
        return ellipse_contains(dx, dy, rx, ry);
    }

    if stroke_thickness > 0.0 {
        let in_stroke = ellipse_contains(dx, dy, rx, ry);

        rx = rect.width / 2.0 - stroke_thickness / 2.0;
        ry = rect.height / 2.0 - stroke_thickness / 2.0;

        let in_inner = ellipse_contains(dx, dy, rx, ry);

        return in_stroke && !in_inner;
    }

    false
}

fn hit_test_ellipse_geometry(
    server_brush: Option<&dyn IBrush>,
    client_pen: Option<&dyn IPen>,
    rect: Rect,
    geometry: &Ref<Geometry>,
) -> IntersectionResult {
    hit_test_geometry(
        server_brush,
        client_pen,
        geometry.platform_impl().as_ref(),
        EllipseGeometry::with_rect(rect).platform_impl().as_ref(),
    )
}

fn ellipse_contains(dx: f64, dy: f64, radius_x: f64, radius_y: f64) -> bool {
    let rx2 = radius_x * radius_x;
    let ry2 = radius_y * radius_y;

    let distance = ry2 * dx * dx + rx2 * dy * dy;

    distance <= rx2 * ry2
}
