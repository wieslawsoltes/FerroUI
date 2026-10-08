use super::{DrawingLog, MockDrawingContextImpl, MockDrawingContextLayerImpl, MockRenderTargetBitmapImpl};
use crate::media::imaging::BitmapInterpolationMode;
use crate::media::{FillRule, GeometryCombineMode, IPen, IntersectionResult, SweepDirection};
use crate::platform::surfaces::IPlatformRenderSurface;
use crate::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, IDrawingContextLayerImpl, IGeometryContext, IGeometryImpl,
    IOptionalFeatureProvider, IPlatformGraphicsContext, IPlatformRenderInterface, IPlatformRenderInterfaceContext,
    IPlatformRenderInterfaceRegion, IRenderTarget, IRenderTargetBitmapImpl, IStreamGeometryContextImpl,
    IStreamGeometryImpl, ITransformedGeometryImpl, IWriteableBitmapImpl, LtrbPixelRect, LtrbRect, PixelFormat,
    PixelFormats, RenderTargetDrawingContextProperties, RenderTargetProperties, RenderTargetSceneInfo,
};
use crate::reactive::IDisposable;
use crate::{FerroLocator, Matrix, PixelSize, Point, Rect, Size, Vector};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::io::Read;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// A platform geometry that behaves like its bounding rectangle.
///
/// As the headless geometries of the reference, rectangle and line
/// geometries also know their corner points (`HeadlessGeometryWithEdgesStub`),
/// and a transformed geometry knows the transformed points of its source
/// (`HeadlessTransformedGeometryStub`): fill intersections between such
/// geometries and with stream geometries use a separating axis test on them.
pub struct MockGeometryImpl {
    bounds: Rect,
    source: Option<(Arc<MockGeometryImpl>, Matrix)>,
    edges: Option<Vec<Point>>,
}

impl MockGeometryImpl {
    pub fn new(bounds: Rect) -> Arc<MockGeometryImpl> {
        Arc::new(MockGeometryImpl { bounds: bounds, source: None, edges: None })
    }

    /// A geometry with the given corner points
    /// (`HeadlessGeometryWithEdgesStub`).
    fn with_edges(bounds: Rect, points: Vec<Point>) -> Arc<MockGeometryImpl> {
        Arc::new(MockGeometryImpl { bounds: bounds, source: None, edges: Some(points) })
    }

    /// `HeadlessRectangleGeometryContextStub`.
    pub fn rectangle(bounds: Rect) -> Arc<MockGeometryImpl> {
        Self::with_edges(bounds, vec![bounds.top_left(), bounds.top_right(), bounds.bottom_left(), bounds.bottom_right()])
    }

    /// `HeadlessLineGeometryContextStub`.
    pub fn line(p1: Point, p2: Point) -> Arc<MockGeometryImpl> {
        let bounds = Rect::from_points(
            Point::new(p1.x.min(p2.x), p1.y.min(p2.y)),
            Point::new(p1.x.max(p2.x), p1.y.max(p2.y)),
        );
        Self::with_edges(bounds, vec![p1, p2])
    }

    /// The points of the geometry when it is a geometry with edges
    /// (`IHeadlessGeometryWithEdges.Points`): its own points, or, for a
    /// transformed geometry, the transformed points of its source (none
    /// when the source has no edges).
    fn edge_points(&self) -> Option<Vec<Point>> {
        match &self.source {
            Some((source, transform)) => Some(
                source.edges.as_ref().map_or_else(Vec::new, |points| {
                    points.iter().map(|p| p.transform(*transform)).collect()
                }),
            ),
            None => self.edges.clone(),
        }
    }
}

/// The points of a geometry with edges, if `geometry` is one of the mock
/// geometries that has them.
fn edge_points_of(geometry: &dyn IGeometryImpl) -> Option<Vec<Point>> {
    geometry.as_any().downcast_ref::<MockGeometryImpl>().and_then(MockGeometryImpl::edge_points)
}

/// `IHeadlessGeometryWithEdges.ProjectionOnAxis`.
fn projection_on_axis(points: &[Point], axis: Vector) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for point in points {
        let p = axis.x * point.x + axis.y * point.y;
        if p < min {
            min = p;
        }
        if p > max {
            max = p;
        }
    }
    (min, max)
}

/// `IHeadlessGeometryWithEdges.GetAxes`, with the reference's choice of the
/// other point of an edge (`(i + i) % Count`).
fn axes(points: &[Point]) -> Vec<Vector> {
    let count = points.len();
    (0..count)
        .map(|i| {
            let point = points[i];
            let other_point = points[(i + i) % count];
            let edge = Vector::new(point.x - other_point.x, point.y - other_point.y);
            Vector::new(-edge.y, edge.x)
        })
        .collect()
}

/// Whether a separating axis of the two point sets exists.
fn separated(a: &[Point], b: &[Point]) -> bool {
    let mut all = axes(a);
    all.extend(axes(b));
    all.into_iter().any(|axis| {
        let (min, max) = projection_on_axis(a, axis);
        let (min2, max2) = projection_on_axis(b, axis);
        max < min2 || max2 < min
    })
}

/// The intersection of the geometry with bounds `a` with the geometry with
/// bounds `b`, from the point of view of `a`: as the headless and Skia
/// geometries of the reference, `a` containing `b` means `b` is fully
/// inside `a`.
fn rect_intersection(a: Rect, b: Rect) -> IntersectionResult {
    if a.width <= 0.0 || a.height <= 0.0 || b.width <= 0.0 || b.height <= 0.0 || !a.intersects(b) {
        IntersectionResult::Empty
    } else if a.contains_rect(b) {
        IntersectionResult::FullyInside
    } else if b.contains_rect(a) {
        IntersectionResult::FullyContains
    } else {
        IntersectionResult::Intersects
    }
}

impl IGeometryImpl for MockGeometryImpl {
    fn bounds(&self) -> Rect {
        self.bounds
    }
    fn contour_length(&self) -> f64 {
        let bounds = self.bounds;
        2.0 * (bounds.width + bounds.height)
    }
    fn get_render_bounds(&self, pen: Option<&dyn IPen>) -> Rect {
        self.bounds.inflate(pen.map_or(0.0, |p| p.thickness()) / 2.0)
    }
    fn get_widened_geometry(&self, pen: &dyn IPen) -> Arc<dyn IGeometryImpl> {
        MockGeometryImpl::new(self.bounds.inflate(pen.thickness() / 2.0))
    }
    fn fill_contains(&self, point: Point) -> bool {
        self.bounds.contains(point)
    }
    fn get_fill_intersection_result(&self, geometry: &dyn IGeometryImpl) -> IntersectionResult {
        // A geometry with edges of its own (not a transformed one, which
        // uses the bounds as the base class of the reference does) tests
        // the edges of another geometry with edges.
        if let (None, Some(points)) = (&self.source, &self.edges) {
            if let Some(other) = edge_points_of(geometry) {
                if separated(points, &other) {
                    return IntersectionResult::Empty;
                }
                let bounds = self.get_render_bounds(None);
                let other_bounds = geometry.get_render_bounds(None);
                if bounds.contains_rect(other_bounds) {
                    return IntersectionResult::FullyInside;
                }
                if other_bounds.contains_rect(bounds) {
                    return IntersectionResult::FullyContains;
                }
                return IntersectionResult::Intersects;
            }
        }
        rect_intersection(self.bounds, geometry.bounds())
    }
    fn intersect(&self, geometry: &dyn IGeometryImpl) -> Option<Arc<dyn IGeometryImpl>> {
        let intersection = self.bounds.intersect(geometry.bounds());
        if intersection.width <= 0.0 || intersection.height <= 0.0 {
            None
        } else {
            Some(MockGeometryImpl::new(intersection))
        }
    }
    fn stroke_contains(&self, pen: Option<&dyn IPen>, point: Point) -> bool {
        let half = pen.map_or(0.0, |p| p.thickness()) / 2.0;
        let bounds = self.bounds;
        bounds.inflate(half).contains(point) && !bounds.deflate(half).contains_exclusive(point)
    }
    fn with_transform(&self, transform: Matrix) -> Arc<dyn ITransformedGeometryImpl> {
        // As the reference, the transform of a transformed geometry is
        // combined with the new one over the same source.
        let (source, transform) = match &self.source {
            Some((source, own)) => (source.clone(), *own * transform),
            None => (
                Arc::new(MockGeometryImpl {
                    bounds: self.bounds,
                    source: None,
                    edges: self.edges.clone(),
                }),
                transform,
            ),
        };
        Arc::new(MockGeometryImpl {
            bounds: source.bounds.transform_to_aabb(transform),
            source: Some((source, transform)),
            edges: None,
        })
    }
    fn try_get_point_at_distance(&self, _distance: f64) -> Option<Point> {
        None
    }
    fn try_get_point_and_tangent_at_distance(&self, _distance: f64) -> Option<(Point, Point)> {
        None
    }
    fn try_get_segment(&self, _start: f64, _stop: f64, _begin: bool) -> Option<Arc<dyn IGeometryImpl>> {
        None
    }
    fn as_transformed_geometry(&self) -> Option<&dyn ITransformedGeometryImpl> {
        self.source.as_ref().map(|_| self as &dyn ITransformedGeometryImpl)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ITransformedGeometryImpl for MockGeometryImpl {
    fn source_geometry(&self) -> Arc<dyn IGeometryImpl> {
        match &self.source {
            Some((source, _)) => source.clone() as Arc<dyn IGeometryImpl>,
            None => MockGeometryImpl::new(self.bounds),
        }
    }
    fn transform(&self) -> Matrix {
        self.source.as_ref().map_or(Matrix::IDENTITY, |(_, transform)| *transform)
    }
}

/// A stream geometry whose bounds are the bounding box of the points
/// written to it; a point is in its fill when it is in one of the triangles
/// of consecutive points.
pub struct MockStreamGeometryImpl {
    points: Arc<Mutex<Vec<Point>>>,
}

impl MockStreamGeometryImpl {
    pub fn new() -> Arc<MockStreamGeometryImpl> {
        Arc::new(MockStreamGeometryImpl { points: Arc::new(Mutex::new(Vec::new())) })
    }

    fn as_rect(&self) -> Arc<MockGeometryImpl> {
        let points = self.points.lock().unwrap();
        let mut bounds: Option<Rect> = None;
        for p in points.iter() {
            let r = Rect::new(p.x, p.y, 0.0, 0.0);
            bounds = Some(match bounds {
                Some(b) => {
                    let left = b.x.min(r.x);
                    let top = b.y.min(r.y);
                    Rect::new(left, top, b.right().max(r.x) - left, b.bottom().max(r.y) - top)
                }
                None => r,
            });
        }
        MockGeometryImpl::new(bounds.unwrap_or_default())
    }
}

impl IGeometryImpl for MockStreamGeometryImpl {
    fn bounds(&self) -> Rect {
        self.as_rect().bounds()
    }
    fn contour_length(&self) -> f64 {
        self.as_rect().contour_length()
    }
    fn get_render_bounds(&self, pen: Option<&dyn IPen>) -> Rect {
        self.as_rect().get_render_bounds(pen)
    }
    fn get_widened_geometry(&self, pen: &dyn IPen) -> Arc<dyn IGeometryImpl> {
        self.as_rect().get_widened_geometry(pen)
    }
    fn fill_contains(&self, point: Point) -> bool {
        // As the headless stream geometry of the reference: the point is
        // tested against the triangles of consecutive points (the geometry
        // is assumed to be convex), with the algorithm from
        // https://www.blackpawn.com/texts/pointinpoly/default.html.
        let points = self.points.lock().unwrap();
        let count = points.len();
        for i in 0..count {
            let a = points[i];
            let b = points[(i + 1) % count];
            let c = points[(i + 2) % count];

            let v0 = c - a;
            let v1 = b - a;
            let v2 = point - a;

            let dot00 = v0.x * v0.x + v0.y * v0.y;
            let dot01 = v0.x * v1.x + v0.y * v1.y;
            let dot02 = v0.x * v2.x + v0.y * v2.y;
            let dot11 = v1.x * v1.x + v1.y * v1.y;
            let dot12 = v1.x * v2.x + v1.y * v2.y;

            let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
            let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
            let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
            if u >= 0.0 && v >= 0.0 && u + v < 1.0 {
                return true;
            }
        }
        false
    }
    fn get_fill_intersection_result(&self, geometry: &dyn IGeometryImpl) -> IntersectionResult {
        // As the headless stream geometry of the reference: against a
        // geometry with edges, a separating axis test on the points.
        if let Some(other) = edge_points_of(geometry) {
            let points = self.points.lock().unwrap();
            return if separated(&points, &other) { IntersectionResult::Empty } else { IntersectionResult::Intersects };
        }
        self.as_rect().get_fill_intersection_result(geometry)
    }
    fn intersect(&self, geometry: &dyn IGeometryImpl) -> Option<Arc<dyn IGeometryImpl>> {
        self.as_rect().intersect(geometry)
    }
    fn stroke_contains(&self, pen: Option<&dyn IPen>, point: Point) -> bool {
        self.as_rect().stroke_contains(pen, point)
    }
    fn with_transform(&self, transform: Matrix) -> Arc<dyn ITransformedGeometryImpl> {
        self.as_rect().with_transform(transform)
    }
    fn try_get_point_at_distance(&self, _distance: f64) -> Option<Point> {
        None
    }
    fn try_get_point_and_tangent_at_distance(&self, _distance: f64) -> Option<(Point, Point)> {
        None
    }
    fn try_get_segment(&self, _start: f64, _stop: f64, _begin: bool) -> Option<Arc<dyn IGeometryImpl>> {
        None
    }
    fn as_stream_geometry(&self) -> Option<&dyn IStreamGeometryImpl> {
        Some(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IStreamGeometryImpl for MockStreamGeometryImpl {
    fn clone_geometry(&self) -> Arc<dyn IStreamGeometryImpl> {
        Arc::new(MockStreamGeometryImpl { points: Arc::new(Mutex::new(self.points.lock().unwrap().clone())) })
    }
    fn open(&self) -> Box<dyn IStreamGeometryContextImpl> {
        self.points.lock().unwrap().clear();
        Box::new(MockStreamGeometryContext { points: self.points.clone() })
    }
}

struct MockStreamGeometryContext {
    points: Arc<Mutex<Vec<Point>>>,
}

impl IGeometryContext for MockStreamGeometryContext {
    fn arc_to(&mut self, point: Point, _: Size, _: f64, _: bool, _: SweepDirection, _: bool) {
        self.points.lock().unwrap().push(point);
    }
    fn begin_figure(&mut self, start_point: Point, _is_filled: bool) {
        self.points.lock().unwrap().push(start_point);
    }
    fn cubic_bezier_to(&mut self, p1: Point, p2: Point, p3: Point, _is_stroked: bool) {
        self.points.lock().unwrap().extend([p1, p2, p3]);
    }
    fn quadratic_bezier_to(&mut self, p1: Point, p2: Point, _is_stroked: bool) {
        self.points.lock().unwrap().extend([p1, p2]);
    }
    fn line_to(&mut self, point: Point, _is_stroked: bool) {
        self.points.lock().unwrap().push(point);
    }
    fn end_figure(&mut self, _is_closed: bool) {}
    fn set_fill_rule(&mut self, _fill_rule: FillRule) {}
    fn dispose(&mut self) {}
}

impl IStreamGeometryContextImpl for MockStreamGeometryContext {}

/// A region that keeps its rectangles in a list.
#[derive(Default)]
pub struct MockRegion {
    rects: RefCell<Vec<LtrbPixelRect>>,
}

impl IPlatformRenderInterfaceRegion for MockRegion {
    fn add_rect(&self, rect: LtrbPixelRect) {
        self.rects.borrow_mut().push(rect);
    }
    fn reset(&self) {
        self.rects.borrow_mut().clear();
    }
    fn is_empty(&self) -> bool {
        self.rects.borrow().is_empty()
    }
    fn bounds(&self) -> LtrbPixelRect {
        let rects = self.rects.borrow();
        let mut iter = rects.iter();
        let Some(first) = iter.next() else { return LtrbPixelRect::default() };
        iter.fold(*first, |acc, r| acc.union(*r))
    }
    fn rects(&self) -> Vec<LtrbPixelRect> {
        self.rects.borrow().clone()
    }
    fn intersects(&self, rect: LtrbRect) -> bool {
        self.rects.borrow().iter().any(|r| {
            LtrbRect::from_rect(Rect::new(
                f64::from(r.left),
                f64::from(r.top),
                f64::from(r.width()),
                f64::from(r.height()),
            ))
            .intersects(rect)
        })
    }
    fn contains(&self, pt: Point) -> bool {
        self.rects.borrow().iter().any(|r| {
            pt.x >= f64::from(r.left) && pt.x < f64::from(r.right) && pt.y >= f64::from(r.top) && pt.y < f64::from(r.bottom)
        })
    }
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A render target whose drawing contexts record to a [`DrawingLog`].
pub struct MockRenderTarget {
    log: DrawingLog,
    pub properties: Cell<RenderTargetProperties>,
    pub disposed: Cell<bool>,
}

impl MockRenderTarget {
    pub fn new(log: DrawingLog) -> Rc<MockRenderTarget> {
        Rc::new(MockRenderTarget { log, properties: Cell::default(), disposed: Cell::new(false) })
    }
}

impl IRenderTarget for MockRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        self.properties.get()
    }
    fn create_drawing_context(
        &self,
        _scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        (Box::new(MockDrawingContextImpl::new(self.log.clone())), RenderTargetDrawingContextProperties::default())
    }
    fn dispose(&self) {
        self.disposed.set(true);
    }
}

/// A backend context that creates mock render targets.
pub struct MockPlatformRenderInterfaceContext {
    log: DrawingLog,
    pub is_lost: Cell<bool>,
}

impl IOptionalFeatureProvider for MockPlatformRenderInterfaceContext {
    fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}

impl IPlatformRenderInterfaceContext for MockPlatformRenderInterfaceContext {
    fn create_render_target(&self, _surfaces: &[std::sync::Arc<dyn IPlatformRenderSurface>]) -> Rc<dyn IRenderTarget> {
        MockRenderTarget::new(self.log.clone())
    }
    fn create_offscreen_render_target(
        &self,
        pixel_size: PixelSize,
        _scaling: Vector,
        _enable_text_antialiasing: bool,
    ) -> Rc<dyn IDrawingContextLayerImpl> {
        Rc::new(MockDrawingContextLayerImpl::new(self.log.clone(), pixel_size))
    }
    fn is_lost(&self) -> bool {
        self.is_lost.get()
    }
    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        None
    }
    fn dispose(&self) {}
}

/// A render interface for tests: rectangle-like geometries, drawing
/// contexts that record to a [`DrawingLog`], no bitmap decoding.
pub struct MockPlatformRenderInterface {
    log: DrawingLog,
}

impl MockPlatformRenderInterface {
    pub fn new(log: DrawingLog) -> Rc<MockPlatformRenderInterface> {
        Rc::new(MockPlatformRenderInterface { log })
    }

    /// The log every drawing context created through this interface records
    /// to.
    pub fn log(&self) -> &DrawingLog {
        &self.log
    }

    /// Registers a new mock render interface in a fresh locator scope.
    /// Dispose the returned scope to unregister it.
    pub fn install() -> (Rc<dyn IDisposable>, Rc<MockPlatformRenderInterface>) {
        let scope = FerroLocator::enter_scope();
        let render_interface = MockPlatformRenderInterface::new(DrawingLog::new());
        FerroLocator::current_mutable().bind::<dyn IPlatformRenderInterface>().to_constant(render_interface.clone());
        (scope, render_interface)
    }
}

fn unsupported<T>() -> std::io::Result<T> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "the mock render interface does not decode bitmaps"))
}

/// A platform glyph run that only knows its bounds: as wide as the glyph
/// advances, from the font's ascent to its descent around the baseline.
pub struct MockGlyphRunImpl {
    font_rendering_em_size: f64,
    baseline_origin: Point,
    bounds: Rect,
}

impl MockGlyphRunImpl {
    /// A platform glyph run with the given bounds, an em size of 12 and its
    /// baseline origin at the top left of the bounds.
    pub fn new(bounds: Rect) -> Arc<MockGlyphRunImpl> {
        Arc::new(MockGlyphRunImpl { font_rendering_em_size: 12.0, baseline_origin: bounds.position(), bounds })
    }
}

impl crate::platform::IGlyphRunImpl for MockGlyphRunImpl {
    fn font_rendering_em_size(&self) -> f64 {
        self.font_rendering_em_size
    }
    fn baseline_origin(&self) -> Point {
        self.baseline_origin
    }
    fn bounds(&self) -> Rect {
        self.bounds
    }
    fn get_intersections(&self, _lower_limit: f32, _upper_limit: f32) -> Vec<f32> {
        Vec::new()
    }
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IPlatformRenderInterface for MockPlatformRenderInterface {
    fn build_glyph_run_geometry(&self, glyph_run: &crate::media::GlyphRun) -> Arc<dyn IGeometryImpl> {
        MockGeometryImpl::new(glyph_run.bounds())
    }
    fn create_glyph_run(
        &self,
        glyph_typeface: &Rc<crate::media::GlyphTypeface>,
        font_rendering_em_size: f64,
        glyph_infos: &[crate::media::text_formatting::GlyphInfo],
        baseline_origin: Point,
    ) -> std::sync::Arc<dyn crate::platform::IGlyphRunImpl> {
        let metrics = glyph_typeface.metrics();
        let scale = font_rendering_em_size / metrics.design_em_height as f64;
        let width: f64 = glyph_infos.iter().map(|glyph| glyph.glyph_advance).sum();
        std::sync::Arc::new(MockGlyphRunImpl {
            font_rendering_em_size,
            baseline_origin,
            bounds: Rect::new(
                baseline_origin.x,
                baseline_origin.y + metrics.ascent as f64 * scale,
                width,
                (metrics.descent - metrics.ascent) as f64 * scale,
            ),
        })
    }
    fn create_ellipse_geometry(&self, rect: Rect) -> Arc<dyn IGeometryImpl> {
        MockGeometryImpl::new(rect)
    }
    fn create_line_geometry(&self, p1: Point, p2: Point) -> Arc<dyn IGeometryImpl> {
        MockGeometryImpl::line(p1, p2)
    }
    fn create_rectangle_geometry(&self, rect: Rect) -> Arc<dyn IGeometryImpl> {
        MockGeometryImpl::rectangle(rect)
    }
    fn create_stream_geometry(&self) -> Arc<dyn IStreamGeometryImpl> {
        MockStreamGeometryImpl::new()
    }
    fn create_geometry_group(&self, _fill_rule: FillRule, children: &[Arc<dyn IGeometryImpl>]) -> Arc<dyn IGeometryImpl> {
        let bounds = children.iter().fold(None, |acc, g| Rect::union_optional(acc, Some(g.bounds())));
        MockGeometryImpl::new(bounds.unwrap_or_default())
    }
    fn create_combined_geometry(
        &self,
        _combine_mode: GeometryCombineMode,
        g1: Arc<dyn IGeometryImpl>,
        g2: Arc<dyn IGeometryImpl>,
    ) -> Arc<dyn IGeometryImpl> {
        MockGeometryImpl::new(g1.bounds().union(g2.bounds()))
    }
    fn create_render_target_bitmap(&self, size: PixelSize, dpi: Vector) -> std::sync::Arc<dyn IRenderTargetBitmapImpl> {
        std::sync::Arc::new(MockRenderTargetBitmapImpl::new(self.log.clone(), size, dpi))
    }
    fn create_writeable_bitmap(
        &self,
        _size: PixelSize,
        _dpi: Vector,
        _format: PixelFormat,
        _alpha_format: AlphaFormat,
    ) -> std::sync::Arc<dyn IWriteableBitmapImpl> {
        panic!("the mock render interface does not create writeable bitmaps")
    }
    fn load_bitmap_from_file(&self, _file_name: &str) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
        unsupported()
    }
    fn load_bitmap(&self, _stream: &mut dyn Read) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
        unsupported()
    }
    fn load_writeable_bitmap_to_width(
        &self,
        _stream: &mut dyn Read,
        _width: i32,
        _interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<std::sync::Arc<dyn IWriteableBitmapImpl>> {
        unsupported()
    }
    fn load_writeable_bitmap_to_height(
        &self,
        _stream: &mut dyn Read,
        _height: i32,
        _interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<std::sync::Arc<dyn IWriteableBitmapImpl>> {
        unsupported()
    }
    fn load_writeable_bitmap_from_file(&self, _file_name: &str) -> std::io::Result<std::sync::Arc<dyn IWriteableBitmapImpl>> {
        unsupported()
    }
    fn load_writeable_bitmap(&self, _stream: &mut dyn Read) -> std::io::Result<std::sync::Arc<dyn IWriteableBitmapImpl>> {
        unsupported()
    }
    fn load_bitmap_to_width(
        &self,
        _stream: &mut dyn Read,
        _width: i32,
        _interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
        unsupported()
    }
    fn load_bitmap_to_height(
        &self,
        _stream: &mut dyn Read,
        _height: i32,
        _interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
        unsupported()
    }
    fn resize_bitmap(
        &self,
        _bitmap_impl: &dyn IBitmapImpl,
        destination_size: PixelSize,
        _interpolation_mode: BitmapInterpolationMode,
    ) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
        std::sync::Arc::new(MockDrawingContextLayerImpl::new(self.log.clone(), destination_size))
    }
    fn load_bitmap_from_pixels(
        &self,
        _format: PixelFormat,
        _alpha_format: AlphaFormat,
        _data: &[u8],
        size: PixelSize,
        _dpi: Vector,
        _stride: i32,
    ) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
        std::sync::Arc::new(MockDrawingContextLayerImpl::new(self.log.clone(), size))
    }
    fn create_backend_context(
        &self,
        _graphics_api_context: Option<Rc<dyn IPlatformGraphicsContext>>,
    ) -> Rc<dyn IPlatformRenderInterfaceContext> {
        Rc::new(MockPlatformRenderInterfaceContext { log: self.log.clone(), is_lost: Cell::new(false) })
    }
    fn supports_individual_round_rects(&self) -> bool {
        false
    }
    fn default_alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }
    fn default_pixel_format(&self) -> PixelFormat {
        PixelFormats::RGBA8888
    }
    fn is_supported_bitmap_pixel_format(&self, _format: PixelFormat) -> bool {
        true
    }
    fn supports_regions(&self) -> bool {
        true
    }
    fn create_region(&self) -> Rc<dyn IPlatformRenderInterfaceRegion> {
        Rc::new(MockRegion::default())
    }
}
