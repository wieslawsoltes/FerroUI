//! Port of `HeadlessPlatformRenderInterface.cs`: a render interface that
//! draws nothing. Its geometries behave like their bounding rectangles, its
//! bitmaps have a size and no content.

use crate::headless_platform_stubs::HeadlessFontManagerStub;
use ferroui_base::media::imaging::{BitmapEncoderOptions, BitmapInterpolationMode};
use ferroui_base::media::text_formatting::GlyphInfo;
use ferroui_base::media::{
    BoxShadows, Color, FillRule, GeometryCombineMode, GlyphRun, GlyphTypeface, IBrush, IPen, IntersectionResult,
    RenderOptions, SweepDirection, TextOptions,
};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, IDrawingContextLayerImpl, IFontManagerImpl, IGeometryContext,
    IGeometryImpl, IGlyphRunImpl, ILockedFramebuffer, IOptionalFeatureProvider, IPlatformGraphicsContext,
    IPlatformRenderInterface, IPlatformRenderInterfaceContext, IPlatformRenderInterfaceRegion, IReadableBitmapImpl,
    IRenderTarget, IRenderTargetBitmapImpl, IStreamGeometryContextImpl, IStreamGeometryImpl,
    ITransformedGeometryImpl, IWriteableBitmapImpl, PixelFormat, PixelFormats, RenderTargetDrawingContextProperties,
    RenderTargetProperties, RenderTargetSceneInfo,
};
use ferroui_base::{FerroLocator, LocatorExtensions, Matrix, PixelSize, Point, Rect, RoundedRect, Size, Vector};
use std::any::{Any, TypeId};
use std::cell::{Cell, OnceCell, RefCell};
use std::io::{Read, Write};
use std::rc::{Rc, Weak};

thread_local! {
    /// The render interfaces `initialize` registered on this thread: see
    /// [`HeadlessPlatformRenderInterface::is_current`].
    static INSTANCES: RefCell<Vec<Weak<HeadlessPlatformRenderInterface>>> = const { RefCell::new(Vec::new()) };
}

pub(crate) struct HeadlessPlatformRenderInterface {
    this: Weak<HeadlessPlatformRenderInterface>,
}

impl HeadlessPlatformRenderInterface {
    fn new() -> Rc<HeadlessPlatformRenderInterface> {
        Rc::new_cyclic(|this| HeadlessPlatformRenderInterface { this: this.clone() })
    }

    /// Whether the render interface of the service locator is the headless
    /// one (`GetService<IPlatformRenderInterface>() is HeadlessPlatformRenderInterface`).
    ///
    /// The service is a trait object without a way back to its type, so the
    /// instances created by [`initialize`](Self::initialize) are remembered
    /// and the service is compared with them by identity.
    pub(crate) fn is_current() -> bool {
        let Some(service) = FerroLocator::current().get_service::<dyn IPlatformRenderInterface>() else {
            return false;
        };
        INSTANCES.with(|instances| {
            let mut instances = instances.borrow_mut();
            instances.retain(|instance| instance.strong_count() > 0);
            instances.iter().any(|instance| std::ptr::addr_eq(instance.as_ptr(), Rc::as_ptr(&service)))
        })
    }

    pub(crate) fn initialize() {
        let instance = HeadlessPlatformRenderInterface::new();
        INSTANCES.with(|instances| instances.borrow_mut().push(Rc::downgrade(&instance)));
        let render_interface: Rc<dyn IPlatformRenderInterface> = instance;
        let font_manager: Rc<dyn IFontManagerImpl> = Rc::new(HeadlessFontManagerStub::new());
        FerroLocator::current_mutable()
            .bind::<dyn IPlatformRenderInterface>()
            .to_constant(render_interface)
            .bind::<dyn IFontManagerImpl>()
            .to_constant(font_manager);
    }

    fn rc(&self) -> Rc<HeadlessPlatformRenderInterface> {
        self.this.upgrade().expect("the render interface is alive while it is used")
    }
}

/// A bitmap of one device-independent pixel at 96 DPI: what the interface
/// answers every request to load a bitmap with.
fn one_pixel_bitmap() -> Rc<HeadlessBitmapStub> {
    Rc::new(HeadlessBitmapStub::from_size(Size::new(1.0, 1.0), Vector::new(96.0, 96.0)))
}

impl IPlatformRenderInterface for HeadlessPlatformRenderInterface {
    fn create_backend_context(
        &self,
        _graphics_api_context: Option<Rc<dyn IPlatformGraphicsContext>>,
    ) -> Rc<dyn IPlatformRenderInterfaceContext> {
        self.rc()
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
        false
    }

    /// # Panics
    /// Always: the headless interface has no regions (`NotSupportedException`).
    fn create_region(&self) -> Rc<dyn IPlatformRenderInterfaceRegion> {
        panic!("Specified method is not supported.");
    }

    fn create_ellipse_geometry(&self, rect: Rect) -> Rc<dyn IGeometryImpl> {
        HeadlessGeometryStub::new(rect)
    }

    fn create_line_geometry(&self, p1: Point, p2: Point) -> Rc<dyn IGeometryImpl> {
        HeadlessGeometryStub::line(p1, p2)
    }

    fn create_rectangle_geometry(&self, rect: Rect) -> Rc<dyn IGeometryImpl> {
        HeadlessGeometryStub::rectangle(rect)
    }

    fn create_stream_geometry(&self) -> Rc<dyn IStreamGeometryImpl> {
        HeadlessStreamingGeometryStub::new()
    }

    fn create_geometry_group(&self, _fill_rule: FillRule, children: &[Rc<dyn IGeometryImpl>]) -> Rc<dyn IGeometryImpl> {
        let mut bounds: Option<Rect> = None;
        for child in children {
            bounds = Some(match bounds {
                Some(bounds) => bounds.union(child.bounds()),
                None => child.bounds(),
            });
        }
        HeadlessGeometryStub::new(bounds.unwrap_or_default())
    }

    fn create_combined_geometry(
        &self,
        _combine_mode: GeometryCombineMode,
        g1: Rc<dyn IGeometryImpl>,
        g2: Rc<dyn IGeometryImpl>,
    ) -> Rc<dyn IGeometryImpl> {
        HeadlessGeometryStub::new(g1.bounds().union(g2.bounds()))
    }

    fn create_render_target_bitmap(&self, size: PixelSize, dpi: Vector) -> Rc<dyn IRenderTargetBitmapImpl> {
        Rc::new(HeadlessBitmapStub::from_pixel_size(size, dpi))
    }

    fn create_writeable_bitmap(
        &self,
        size: PixelSize,
        dpi: Vector,
        _format: PixelFormat,
        _alpha_format: AlphaFormat,
    ) -> Rc<dyn IWriteableBitmapImpl> {
        Rc::new(HeadlessBitmapStub::from_pixel_size(size, dpi))
    }

    fn load_bitmap_from_file(&self, _file_name: &str) -> std::io::Result<Rc<dyn IBitmapImpl>> {
        Ok(one_pixel_bitmap())
    }

    fn load_bitmap(&self, _stream: &mut dyn Read) -> std::io::Result<Rc<dyn IBitmapImpl>> {
        Ok(one_pixel_bitmap())
    }

    fn load_writeable_bitmap_to_width(
        &self,
        _stream: &mut dyn Read,
        _width: i32,
        _interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<Rc<dyn IWriteableBitmapImpl>> {
        Ok(one_pixel_bitmap())
    }

    fn load_writeable_bitmap_to_height(
        &self,
        _stream: &mut dyn Read,
        _height: i32,
        _interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<Rc<dyn IWriteableBitmapImpl>> {
        Ok(one_pixel_bitmap())
    }

    fn load_writeable_bitmap_from_file(&self, _file_name: &str) -> std::io::Result<Rc<dyn IWriteableBitmapImpl>> {
        Ok(one_pixel_bitmap())
    }

    fn load_writeable_bitmap(&self, _stream: &mut dyn Read) -> std::io::Result<Rc<dyn IWriteableBitmapImpl>> {
        Ok(one_pixel_bitmap())
    }

    fn load_bitmap_from_pixels(
        &self,
        _format: PixelFormat,
        _alpha_format: AlphaFormat,
        _data: &[u8],
        _size: PixelSize,
        _dpi: Vector,
        _stride: i32,
    ) -> Rc<dyn IBitmapImpl> {
        one_pixel_bitmap()
    }

    fn load_bitmap_to_width(
        &self,
        _stream: &mut dyn Read,
        width: i32,
        _interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<Rc<dyn IBitmapImpl>> {
        let width = f64::from(width);
        Ok(Rc::new(HeadlessBitmapStub::from_size(Size::new(width, width), Vector::new(96.0, 96.0))))
    }

    fn load_bitmap_to_height(
        &self,
        _stream: &mut dyn Read,
        height: i32,
        _interpolation_mode: BitmapInterpolationMode,
    ) -> std::io::Result<Rc<dyn IBitmapImpl>> {
        let height = f64::from(height);
        Ok(Rc::new(HeadlessBitmapStub::from_size(Size::new(height, height), Vector::new(96.0, 96.0))))
    }

    fn resize_bitmap(
        &self,
        _bitmap_impl: &dyn IBitmapImpl,
        destination_size: PixelSize,
        _interpolation_mode: BitmapInterpolationMode,
    ) -> Rc<dyn IBitmapImpl> {
        Rc::new(HeadlessBitmapStub::from_pixel_size(destination_size, Vector::new(96.0, 96.0)))
    }

    fn build_glyph_run_geometry(&self, glyph_run: &GlyphRun) -> Rc<dyn IGeometryImpl> {
        HeadlessGeometryStub::new(glyph_run.bounds())
    }

    fn create_glyph_run(
        &self,
        glyph_typeface: &Rc<GlyphTypeface>,
        font_rendering_em_size: f64,
        _glyph_infos: &[GlyphInfo],
        baseline_origin: Point,
    ) -> Rc<dyn IGlyphRunImpl> {
        Rc::new(HeadlessGlyphRunStub::new(glyph_typeface.clone(), font_rendering_em_size, baseline_origin))
    }
}

impl IOptionalFeatureProvider for HeadlessPlatformRenderInterface {
    fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}

impl IPlatformRenderInterfaceContext for HeadlessPlatformRenderInterface {
    fn create_render_target(&self, _surfaces: &[Rc<dyn IPlatformRenderSurface>]) -> Rc<dyn IRenderTarget> {
        Rc::new(HeadlessRenderTarget)
    }

    fn create_offscreen_render_target(
        &self,
        pixel_size: PixelSize,
        scaling: Vector,
        _enable_text_antialiasing: bool,
    ) -> Rc<dyn IDrawingContextLayerImpl> {
        Rc::new(HeadlessBitmapStub::from_pixel_size(pixel_size, scaling * 96.0))
    }

    fn is_lost(&self) -> bool {
        false
    }

    fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
        None
    }

    fn dispose(&self) {}
}

pub(crate) struct HeadlessGlyphRunStub {
    glyph_typeface: Rc<GlyphTypeface>,
    font_rendering_em_size: f64,
    baseline_origin: Point,
}

impl HeadlessGlyphRunStub {
    pub(crate) fn new(glyph_typeface: Rc<GlyphTypeface>, font_rendering_em_size: f64, baseline_origin: Point) -> Self {
        Self { glyph_typeface, font_rendering_em_size, baseline_origin }
    }

    #[allow(dead_code)] // public property of the original, which nothing reads
    pub(crate) fn glyph_typeface(&self) -> &Rc<GlyphTypeface> {
        &self.glyph_typeface
    }
}

impl IGlyphRunImpl for HeadlessGlyphRunStub {
    fn font_rendering_em_size(&self) -> f64 {
        self.font_rendering_em_size
    }

    fn baseline_origin(&self) -> Point {
        self.baseline_origin
    }

    /// As upstream, the bounds are never set.
    fn bounds(&self) -> Rect {
        Rect::default()
    }

    fn get_intersections(&self, _lower_limit: f32, _upper_limit: f32) -> Vec<f32> {
        Vec::new()
    }

    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// --- geometries -----------------------------------------------------------------------------
//
// The class hierarchy of the original (`HeadlessGeometryStub`, its subclasses with edges and the
// transformed one) is two structs here: `HeadlessGeometryStub`, with or without corner points,
// and `HeadlessTransformedGeometryStub`. The members of the base class are the `base_*`
// functions over the bounds.

/// `HeadlessGeometryStub.GetRenderBounds`.
fn base_get_render_bounds(bounds: Rect, pen: Option<&dyn IPen>) -> Rect {
    match pen {
        None => bounds,
        Some(pen) => bounds.inflate(pen.thickness() / 2.0),
    }
}

/// `HeadlessGeometryStub.Intersect`.
fn base_intersect(bounds: Rect, geometry: &dyn IGeometryImpl) -> Rc<HeadlessGeometryStub> {
    let mut intersection = geometry.bounds().intersect(bounds);
    if intersection == Rect::default() {
        // In the case that a 0-width or 0-height geometry, like a line is being tested
        let rect1 = bounds;
        let rect2 = geometry.bounds();
        let new_left = if rect1.x > rect2.x { rect1.x } else { rect2.x };
        let new_top = if rect1.y > rect2.y { rect1.y } else { rect2.y };
        let new_right = if rect1.right() < rect2.right() { rect1.right() } else { rect2.right() };
        let new_bottom = if rect1.bottom() < rect2.bottom() { rect1.bottom() } else { rect2.bottom() };

        if new_right >= new_left && new_bottom >= new_top {
            intersection = Rect::new(new_left, new_top, new_right - new_left, new_bottom - new_top);
        }
    }

    HeadlessGeometryStub::new(intersection)
}

/// The comparison of the render bounds that ends the fill intersection
/// tests of the original.
fn bounds_intersection_result(bounds: Rect, geometry: &dyn IGeometryImpl) -> IntersectionResult {
    let other_bounds = geometry.get_render_bounds(None);

    if bounds.contains_rect(other_bounds) {
        return IntersectionResult::FullyInside;
    }

    if other_bounds.contains_rect(bounds) {
        return IntersectionResult::FullyContains;
    }

    IntersectionResult::Intersects
}

/// `HeadlessGeometryStub.GetFillIntersectionResult`.
fn base_get_fill_intersection_result(bounds: Rect, geometry: &dyn IGeometryImpl) -> IntersectionResult {
    let intersection = base_intersect(bounds, geometry).bounds();

    if Size::new(intersection.width, intersection.height) != Size::default() {
        return bounds_intersection_result(base_get_render_bounds(bounds, None), geometry);
    }

    IntersectionResult::Empty
}

/// `IHeadlessGeometryWithEdges.ProjectionOnAxis`.
fn projection_on_axis(points: &[Point], axis: Vector) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;

    for point in points {
        let p = Vector::dot(axis, Vector::new(point.x, point.y));

        if p < min {
            min = p;
        }

        if p > max {
            max = p;
        }
    }

    (min, max)
}

/// `IHeadlessGeometryWithEdges.GetAxes`, with the original's choice of the
/// other point of an edge (`(i + i) % Count`).
fn get_axes(points: &[Point]) -> Vec<Vector> {
    let mut axes = Vec::new();

    for i in 0..points.len() {
        let point = points[i];
        let other_point = points[(i + i) % points.len()];
        let edge = Vector::new(point.x - other_point.x, point.y - other_point.y);
        axes.push(Vector::new(-edge.y, edge.x));
    }

    axes
}

/// Whether an axis separates the two point sets: the loop over the axes of
/// both geometries of the original.
fn separated(points: &[Point], other: &[Point]) -> bool {
    let mut axes = get_axes(points);
    axes.extend(get_axes(other));

    for axis in axes {
        let (min, max) = projection_on_axis(points, axis);
        let projection2 = projection_on_axis(other, axis);

        if max < projection2.0 || projection2.1 < min {
            return true;
        }
    }

    false
}

/// `geometry is IHeadlessGeometryWithEdges`: the points of a geometry with
/// edges (a line, a rectangle, a transformed geometry).
fn edge_points_of(geometry: &dyn IGeometryImpl) -> Option<Vec<Point>> {
    if let Some(stub) = geometry.as_any().downcast_ref::<HeadlessGeometryStub>() {
        return stub.points.clone();
    }
    if let Some(transformed) = geometry.as_any().downcast_ref::<HeadlessTransformedGeometryStub>() {
        return Some(transformed.points());
    }
    None
}

/// `HeadlessGeometryStub`, and with points `HeadlessGeometryWithEdgesStub`
/// (`HeadlessLineGeometryContextStub`, `HeadlessRectangleGeometryContextStub`).
struct HeadlessGeometryStub {
    this: Weak<HeadlessGeometryStub>,
    bounds: Rect,
    points: Option<Vec<Point>>,
}

impl HeadlessGeometryStub {
    fn create(bounds: Rect, points: Option<Vec<Point>>) -> Rc<HeadlessGeometryStub> {
        Rc::new_cyclic(|this| HeadlessGeometryStub { this: this.clone(), bounds, points })
    }

    fn new(bounds: Rect) -> Rc<HeadlessGeometryStub> {
        Self::create(bounds, None)
    }

    /// `HeadlessLineGeometryContextStub`.
    fn line(p1: Point, p2: Point) -> Rc<HeadlessGeometryStub> {
        let bounds =
            Rect::from_points(Point::new(p1.x.min(p2.x), p1.y.min(p2.y)), Point::new(p1.x.max(p2.x), p1.y.max(p2.y)));
        Self::create(bounds, Some(vec![p1, p2]))
    }

    /// `HeadlessRectangleGeometryContextStub`.
    fn rectangle(bounds: Rect) -> Rc<HeadlessGeometryStub> {
        Self::create(
            bounds,
            Some(vec![bounds.top_left(), bounds.top_right(), bounds.bottom_left(), bounds.bottom_right()]),
        )
    }

    fn rc(&self) -> Rc<HeadlessGeometryStub> {
        self.this.upgrade().expect("the geometry is alive while it is used")
    }
}

impl IGeometryImpl for HeadlessGeometryStub {
    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn contour_length(&self) -> f64 {
        0.0
    }

    fn fill_contains(&self, point: Point) -> bool {
        self.bounds.contains(point)
    }

    fn get_render_bounds(&self, pen: Option<&dyn IPen>) -> Rect {
        base_get_render_bounds(self.bounds, pen)
    }

    fn get_widened_geometry(&self, _pen: &dyn IPen) -> Rc<dyn IGeometryImpl> {
        self.rc()
    }

    fn stroke_contains(&self, _pen: Option<&dyn IPen>, _point: Point) -> bool {
        false
    }

    fn intersect(&self, geometry: &dyn IGeometryImpl) -> Option<Rc<dyn IGeometryImpl>> {
        let intersection: Rc<dyn IGeometryImpl> = base_intersect(self.bounds, geometry);
        Some(intersection)
    }

    fn with_transform(&self, transform: Matrix) -> Rc<dyn ITransformedGeometryImpl> {
        let source: Rc<dyn IGeometryImpl> = self.rc();
        HeadlessTransformedGeometryStub::new(source, transform)
    }

    fn try_get_point_at_distance(&self, _distance: f64) -> Option<Point> {
        None
    }

    fn try_get_point_and_tangent_at_distance(&self, _distance: f64) -> Option<(Point, Point)> {
        None
    }

    fn try_get_segment(
        &self,
        _start_distance: f64,
        _stop_distance: f64,
        _start_on_begin_figure: bool,
    ) -> Option<Rc<dyn IGeometryImpl>> {
        None
    }

    fn get_fill_intersection_result(&self, geometry: &dyn IGeometryImpl) -> IntersectionResult {
        // `HeadlessGeometryWithEdgesStub.GetFillIntersectionResult`.
        if let Some(points) = &self.points {
            if let Some(other) = edge_points_of(geometry) {
                if separated(points, &other) {
                    return IntersectionResult::Empty;
                }

                return bounds_intersection_result(self.get_render_bounds(None), geometry);
            }
        }

        base_get_fill_intersection_result(self.bounds, geometry)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `HeadlessTransformedGeometryStub`.
struct HeadlessTransformedGeometryStub {
    this: Weak<HeadlessTransformedGeometryStub>,
    bounds: Rect,
    source_geometry: Rc<dyn IGeometryImpl>,
    transform: Matrix,
    points: OnceCell<Vec<Point>>,
}

impl HeadlessTransformedGeometryStub {
    fn new(b: Rc<dyn IGeometryImpl>, transform: Matrix) -> Rc<HeadlessTransformedGeometryStub> {
        // `Fix`: the transform of a transformed geometry is combined with the new one over the
        // same source.
        let (b, transform) = match b.as_any().downcast_ref::<HeadlessTransformedGeometryStub>() {
            Some(transformed) => (transformed.source_geometry.clone(), transformed.transform * transform),
            None => (b, transform),
        };
        let bounds = b.bounds().transform_to_aabb(transform);

        Rc::new_cyclic(|this| HeadlessTransformedGeometryStub {
            this: this.clone(),
            bounds,
            source_geometry: b,
            transform,
            points: OnceCell::new(),
        })
    }

    fn rc(&self) -> Rc<HeadlessTransformedGeometryStub> {
        self.this.upgrade().expect("the geometry is alive while it is used")
    }

    /// `Points`: the transformed points of a source with edges; none
    /// otherwise.
    fn points(&self) -> Vec<Point> {
        match edge_points_of(&*self.source_geometry) {
            Some(points) => self
                .points
                .get_or_init(|| points.iter().map(|point| point.transform(self.transform)).collect())
                .clone(),
            None => Vec::new(),
        }
    }
}

impl IGeometryImpl for HeadlessTransformedGeometryStub {
    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn contour_length(&self) -> f64 {
        0.0
    }

    fn fill_contains(&self, point: Point) -> bool {
        self.bounds.contains(point)
    }

    fn get_render_bounds(&self, pen: Option<&dyn IPen>) -> Rect {
        base_get_render_bounds(self.bounds, pen)
    }

    fn get_widened_geometry(&self, _pen: &dyn IPen) -> Rc<dyn IGeometryImpl> {
        self.rc()
    }

    fn stroke_contains(&self, _pen: Option<&dyn IPen>, _point: Point) -> bool {
        false
    }

    fn intersect(&self, geometry: &dyn IGeometryImpl) -> Option<Rc<dyn IGeometryImpl>> {
        let intersection: Rc<dyn IGeometryImpl> = base_intersect(self.bounds, geometry);
        Some(intersection)
    }

    fn with_transform(&self, transform: Matrix) -> Rc<dyn ITransformedGeometryImpl> {
        let source: Rc<dyn IGeometryImpl> = self.rc();
        HeadlessTransformedGeometryStub::new(source, transform)
    }

    fn try_get_point_at_distance(&self, _distance: f64) -> Option<Point> {
        None
    }

    fn try_get_point_and_tangent_at_distance(&self, _distance: f64) -> Option<(Point, Point)> {
        None
    }

    fn try_get_segment(
        &self,
        _start_distance: f64,
        _stop_distance: f64,
        _start_on_begin_figure: bool,
    ) -> Option<Rc<dyn IGeometryImpl>> {
        None
    }

    // The transformed geometry has points, but the test with edges belongs to
    // `HeadlessGeometryWithEdgesStub`, which it does not derive from.
    fn get_fill_intersection_result(&self, geometry: &dyn IGeometryImpl) -> IntersectionResult {
        base_get_fill_intersection_result(self.bounds, geometry)
    }

    fn as_transformed_geometry(&self) -> Option<&dyn ITransformedGeometryImpl> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ITransformedGeometryImpl for HeadlessTransformedGeometryStub {
    fn source_geometry(&self) -> Rc<dyn IGeometryImpl> {
        self.source_geometry.clone()
    }

    fn transform(&self) -> Matrix {
        self.transform
    }
}

/// `HeadlessStreamingGeometryStub`.
struct HeadlessStreamingGeometryStub {
    this: Weak<HeadlessStreamingGeometryStub>,
    /// `Bounds` of the base class, set when the context is disposed.
    bounds: Rc<Cell<Rect>>,
    /// The points of the one context of the geometry.
    points: Rc<RefCell<Vec<Point>>>,
}

impl HeadlessStreamingGeometryStub {
    fn new() -> Rc<HeadlessStreamingGeometryStub> {
        Rc::new_cyclic(|this| HeadlessStreamingGeometryStub {
            this: this.clone(),
            bounds: Rc::new(Cell::new(Rect::default())),
            points: Rc::new(RefCell::new(Vec::new())),
        })
    }

    fn rc(&self) -> Rc<HeadlessStreamingGeometryStub> {
        self.this.upgrade().expect("the geometry is alive while it is used")
    }
}

impl IGeometryImpl for HeadlessStreamingGeometryStub {
    fn bounds(&self) -> Rect {
        self.bounds.get()
    }

    fn contour_length(&self) -> f64 {
        0.0
    }

    /// `HeadlessStreamingGeometryContextStub.FillContains(Point)`.
    fn fill_contains(&self, point: Point) -> bool {
        // Use the algorithm from https://www.blackpawn.com/texts/pointinpoly/default.html
        // to determine if the point is in the geometry (since it will always be convex in this situation)
        let points = self.points.borrow();
        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            let c = points[(i + 2) % points.len()];

            // The difference of two points is a point: the vectors of the original are built
            // from its coordinates.
            let to_vector = |p: Point| Vector::new(p.x, p.y);
            let v0 = to_vector(c - a);
            let v1 = to_vector(b - a);
            let v2 = to_vector(point - a);

            let dot00 = v0 * v0;
            let dot01 = v0 * v1;
            let dot02 = v0 * v2;
            let dot11 = v1 * v1;
            let dot12 = v1 * v2;

            let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);
            let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
            let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
            if u >= 0.0 && v >= 0.0 && u + v < 1.0 {
                return true;
            }
        }
        false
    }

    fn get_render_bounds(&self, pen: Option<&dyn IPen>) -> Rect {
        base_get_render_bounds(self.bounds.get(), pen)
    }

    fn get_widened_geometry(&self, _pen: &dyn IPen) -> Rc<dyn IGeometryImpl> {
        self.rc()
    }

    fn stroke_contains(&self, _pen: Option<&dyn IPen>, _point: Point) -> bool {
        false
    }

    fn intersect(&self, geometry: &dyn IGeometryImpl) -> Option<Rc<dyn IGeometryImpl>> {
        let intersection: Rc<dyn IGeometryImpl> = base_intersect(self.bounds.get(), geometry);
        Some(intersection)
    }

    fn with_transform(&self, transform: Matrix) -> Rc<dyn ITransformedGeometryImpl> {
        let source: Rc<dyn IGeometryImpl> = self.rc();
        HeadlessTransformedGeometryStub::new(source, transform)
    }

    fn try_get_point_at_distance(&self, _distance: f64) -> Option<Point> {
        None
    }

    fn try_get_point_and_tangent_at_distance(&self, _distance: f64) -> Option<(Point, Point)> {
        None
    }

    fn try_get_segment(
        &self,
        _start_distance: f64,
        _stop_distance: f64,
        _start_on_begin_figure: bool,
    ) -> Option<Rc<dyn IGeometryImpl>> {
        None
    }

    fn get_fill_intersection_result(&self, geometry: &dyn IGeometryImpl) -> IntersectionResult {
        // `HeadlessStreamingGeometryContextStub.FillContains(IHeadlessGeometryWithEdges)`.
        if let Some(other) = edge_points_of(geometry) {
            let points = self.points.borrow();
            return if separated(&points, &other) { IntersectionResult::Empty } else { IntersectionResult::Intersects };
        }

        base_get_fill_intersection_result(self.bounds.get(), geometry)
    }

    fn as_stream_geometry(&self) -> Option<&dyn IStreamGeometryImpl> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IStreamGeometryImpl for HeadlessStreamingGeometryStub {
    /// As upstream, the clone is the geometry itself.
    fn clone_geometry(&self) -> Rc<dyn IStreamGeometryImpl> {
        self.rc()
    }

    /// As upstream, every call returns the one context of the geometry:
    /// the points written through it accumulate.
    fn open(&self) -> Box<dyn IStreamGeometryContextImpl> {
        Box::new(HeadlessStreamingGeometryContextStub { bounds: self.bounds.clone(), points: self.points.clone() })
    }
}

/// `HeadlessStreamingGeometryContextStub`: a handle to the points and the
/// bounds of its geometry.
struct HeadlessStreamingGeometryContextStub {
    bounds: Rc<Cell<Rect>>,
    points: Rc<RefCell<Vec<Point>>>,
}

impl HeadlessStreamingGeometryContextStub {
    fn track(&mut self, pt: Point) {
        self.points.borrow_mut().push(pt);
    }

    fn calculate_bounds(&self) -> Rect {
        let mut left = f64::MAX;
        let mut right = f64::MIN;
        let mut top = f64::MAX;
        let mut bottom = f64::MIN;

        for p in self.points.borrow().iter() {
            left = p.x.min(left);
            right = p.x.max(right);
            top = p.y.min(top);
            bottom = p.y.max(bottom);
        }

        Rect::from_points(Point::new(left, top), Point::new(right, bottom))
    }
}

impl IGeometryContext for HeadlessStreamingGeometryContextStub {
    fn arc_to(
        &mut self,
        point: Point,
        _size: Size,
        _rotation_angle: f64,
        _is_large_arc: bool,
        _sweep_direction: SweepDirection,
        _is_stroked: bool,
    ) {
        self.track(point);
    }

    fn begin_figure(&mut self, start_point: Point, _is_filled: bool) {
        self.track(start_point);
    }

    fn cubic_bezier_to(&mut self, point1: Point, point2: Point, point3: Point, _is_stroked: bool) {
        self.track(point1);
        self.track(point2);
        self.track(point3);
    }

    fn quadratic_bezier_to(&mut self, control: Point, end_point: Point, _is_stroked: bool) {
        self.track(control);
        self.track(end_point);
    }

    fn line_to(&mut self, point: Point, _is_stroked: bool) {
        self.track(point);
    }

    fn end_figure(&mut self, _is_closed: bool) {
        self.dispose();
    }

    fn set_fill_rule(&mut self, _fill_rule: FillRule) {}

    fn dispose(&mut self) {
        self.bounds.set(self.calculate_bounds());
    }
}

impl IStreamGeometryContextImpl for HeadlessStreamingGeometryContextStub {}

// --- bitmaps and drawing contexts -----------------------------------------------------------

/// `HeadlessBitmapStub`: a bitmap, a layer, a writeable bitmap and a render
/// target bitmap with a size and no content.
struct HeadlessBitmapStub {
    #[allow(dead_code)] // public property of the original, which nothing reads
    size: Size,
    dpi: Vector,
    pixel_size: PixelSize,
    version: Cell<i32>,
}

impl HeadlessBitmapStub {
    fn from_size(size: Size, dpi: Vector) -> HeadlessBitmapStub {
        let pixel = size * (dpi / 96.0);
        HeadlessBitmapStub {
            size,
            dpi,
            pixel_size: PixelSize::new((pixel.width as i32).max(1), (pixel.height as i32).max(1)),
            version: Cell::new(0),
        }
    }

    fn from_pixel_size(size: PixelSize, dpi: Vector) -> HeadlessBitmapStub {
        HeadlessBitmapStub { pixel_size: size, dpi, size: size.to_size_with_dpi_vector(dpi), version: Cell::new(0) }
    }
}

impl IBitmapImpl for HeadlessBitmapStub {
    fn dpi(&self) -> Vector {
        self.dpi
    }

    fn pixel_size(&self) -> PixelSize {
        self.pixel_size
    }

    fn version(&self) -> i32 {
        self.version.get()
    }

    fn save(&self, _stream: &mut dyn Write, _options: &BitmapEncoderOptions) -> std::io::Result<()> {
        Ok(())
    }

    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        Some(self)
    }
}

impl IReadableBitmapImpl for HeadlessBitmapStub {
    fn format(&self) -> Option<PixelFormat> {
        Some(PixelFormats::RGBA8888)
    }

    fn alpha_format(&self) -> Option<AlphaFormat> {
        Some(AlphaFormat::Premul)
    }

    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        self.version.set(self.version.get() + 1);
        let width = self.pixel_size.width.max(0) as usize;
        let height = self.pixel_size.height.max(0) as usize;
        Rc::new(HeadlessLockedFramebuffer {
            // Upstream allocates unmanaged memory and frees it when the framebuffer is disposed.
            memory: RefCell::new(vec![0u8; width * height * 4]),
            size: self.pixel_size,
            row_bytes: self.pixel_size.width * 4,
            dpi: self.dpi,
        })
    }
}

impl IWriteableBitmapImpl for HeadlessBitmapStub {}

impl IRenderTargetBitmapImpl for HeadlessBitmapStub {
    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        Box::new(HeadlessDrawingContextStub::new())
    }
}

impl IDrawingContextLayerImpl for HeadlessBitmapStub {
    fn blit(&self, _context: &mut dyn IDrawingContextImpl) {}

    fn can_blit(&self) -> bool {
        false
    }

    fn is_corrupted(&self) -> bool {
        false
    }

    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        Box::new(HeadlessDrawingContextStub::new())
    }
}

/// The memory a headless bitmap hands out when it is locked
/// (`LockedFramebuffer` over memory of its own).
struct HeadlessLockedFramebuffer {
    memory: RefCell<Vec<u8>>,
    size: PixelSize,
    row_bytes: i32,
    dpi: Vector,
}

impl ILockedFramebuffer for HeadlessLockedFramebuffer {
    fn address(&self) -> *mut u8 {
        self.memory.borrow_mut().as_mut_ptr()
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        access(&mut self.memory.borrow_mut());
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn row_bytes(&self) -> i32 {
        self.row_bytes
    }

    fn dpi(&self) -> Vector {
        self.dpi
    }

    fn format(&self) -> PixelFormat {
        PixelFormats::RGBA8888
    }

    fn alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }

    fn dispose(&self) {}
}

pub(crate) struct HeadlessDrawingContextStub {
    transform: Matrix,
}

impl HeadlessDrawingContextStub {
    pub(crate) fn new() -> Self {
        // `Matrix Transform { get; set; }`: the default value of the struct, not the identity.
        Self { transform: Matrix::default() }
    }
}

impl IDrawingContextImpl for HeadlessDrawingContextStub {
    fn transform(&self) -> Matrix {
        self.transform
    }

    fn set_transform(&mut self, value: Matrix) {
        self.transform = value;
    }

    fn clear(&mut self, _color: Color) {}

    fn draw_bitmap(&mut self, _source: &dyn IBitmapImpl, _opacity: f64, _source_rect: Rect, _dest_rect: Rect) {}

    fn draw_bitmap_with_mask(
        &mut self,
        _source: &dyn IBitmapImpl,
        _opacity_mask: &dyn IBrush,
        _opacity_mask_rect: Rect,
        _dest_rect: Rect,
    ) {
    }

    fn draw_line(&mut self, _pen: Option<&dyn IPen>, _p1: Point, _p2: Point) {}

    fn draw_geometry(&mut self, _brush: Option<&dyn IBrush>, _pen: Option<&dyn IPen>, _geometry: &dyn IGeometryImpl) {}

    fn draw_rectangle(
        &mut self,
        _brush: Option<&dyn IBrush>,
        _pen: Option<&dyn IPen>,
        _rect: RoundedRect,
        _box_shadows: &BoxShadows,
    ) {
    }

    fn draw_region(
        &mut self,
        _brush: Option<&dyn IBrush>,
        _pen: Option<&dyn IPen>,
        _region: &dyn IPlatformRenderInterfaceRegion,
    ) {
    }

    fn draw_ellipse(&mut self, _brush: Option<&dyn IBrush>, _pen: Option<&dyn IPen>, _rect: Rect) {}

    fn draw_glyph_run(&mut self, _foreground: Option<&dyn IBrush>, _glyph_run: &dyn IGlyphRunImpl) {}

    fn create_layer(&mut self, size: PixelSize) -> Rc<dyn IDrawingContextLayerImpl> {
        Rc::new(HeadlessBitmapStub::from_pixel_size(size, Vector::new(96.0, 96.0)))
    }

    fn push_clip(&mut self, _clip: Rect) {}

    fn push_clip_rounded(&mut self, _clip: RoundedRect) {}

    fn push_clip_region(&mut self, _region: &dyn IPlatformRenderInterfaceRegion) {}

    fn pop_clip(&mut self) {}

    fn push_layer(&mut self, _bounds: Rect) {}

    fn pop_layer(&mut self) {}

    fn push_opacity(&mut self, _opacity: f64, _bounds: Option<Rect>) {}

    fn pop_opacity(&mut self) {}

    fn push_opacity_mask(&mut self, _mask: &dyn IBrush, _bounds: Rect) {}

    fn pop_opacity_mask(&mut self) {}

    fn push_geometry_clip(&mut self, _clip: &dyn IGeometryImpl) {}

    fn pop_geometry_clip(&mut self) {}

    fn push_render_options(&mut self, _render_options: RenderOptions) {}

    fn pop_render_options(&mut self) {}

    fn push_text_options(&mut self, _text_options: TextOptions) {
        // No-op in headless stub
    }

    fn pop_text_options(&mut self) {
        // No-op in headless stub
    }

    fn get_feature(&mut self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn dispose(&mut self) {}
}

struct HeadlessRenderTarget;

impl IRenderTarget for HeadlessRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties::default()
    }

    fn create_drawing_context(
        &self,
        _scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        (Box::new(HeadlessDrawingContextStub::new()), RenderTargetDrawingContextProperties::default())
    }

    fn dispose(&self) {}
}
