use crate::combined_geometry_impl::CombinedGeometryImpl;
use crate::helpers::path_measure::PathMeasure;
use crate::helpers::{path_helper, pen_helper};
use crate::stream_geometry_impl::StreamGeometryImpl;
use crate::transformed_geometry_impl::TransformedGeometryImpl;
use crate::vello_extensions::{closed_path, to_kurbo_point, to_point, to_rect};
use ferroui_base::media::{FillRule, GeometryCombineMode, IPen, IntersectionResult};
use ferroui_base::platform::{IGeometryImpl, ITransformedGeometryImpl};
use ferroui_base::{Matrix, Point, Rect};
use kurbo::{BezPath, Shape};
use std::sync::{Arc, Mutex, Weak};

/// A path of a geometry with the rule it is filled by.
///
/// A path of kurbo is a list of elements; the fill rule, which a path of
/// Skia carries itself, travels beside it. The elements are shared: a copy
/// of the value is a counted reference.
#[derive(Clone, Debug)]
pub struct VelloPath {
    elements: Arc<BezPath>,
    fill_rule: FillRule,
}

impl VelloPath {
    /// Wraps a path that is filled by `fill_rule`.
    pub fn new(path: BezPath, fill_rule: FillRule) -> Self {
        Self { elements: Arc::new(path), fill_rule }
    }

    /// An empty path with the fill rule of a new stream geometry.
    pub fn empty() -> Self {
        Self::new(BezPath::new(), FillRule::EvenOdd)
    }

    /// The elements of the path.
    pub fn path(&self) -> &BezPath {
        &self.elements
    }

    /// The rule the path is filled by.
    pub fn fill_rule(&self) -> FillRule {
        self.fill_rule
    }

    /// The same elements with another fill rule.
    pub fn with_fill_rule(&self, fill_rule: FillRule) -> Self {
        Self { elements: self.elements.clone(), fill_rule }
    }

    /// The path with a transform applied.
    pub fn transformed(&self, transform: kurbo::Affine) -> Self {
        let mut path = (*self.elements).clone();
        path.apply_affine(transform);
        Self::new(path, self.fill_rule)
    }

    /// The tight bounds of the path: of its curves, not of their control
    /// points. A path without a segment has empty bounds at the origin.
    ///
    /// A path that has only lines has the bounds of all its points, the
    /// points of its moves included, also of a move no segment follows: as
    /// the tight bounds of the Skia backend (its `tight_bounds`), which are
    /// upstream's. Upstream's render tests have paths that end in moves to
    /// give a shape its extent (`M 10,190 L 190,10 M0,0M200,200`).
    pub fn tight_bounds(&self) -> Rect {
        let mut has_line = false;
        let mut only_lines = true;
        for element in self.elements.elements() {
            match element {
                kurbo::PathEl::LineTo(_) => has_line = true,
                kurbo::PathEl::MoveTo(_) | kurbo::PathEl::ClosePath => {}
                kurbo::PathEl::QuadTo(..) | kurbo::PathEl::CurveTo(..) => only_lines = false,
            }
        }

        if has_line && only_lines {
            let mut bounds: Option<kurbo::Rect> = None;
            for element in self.elements.elements() {
                if let kurbo::PathEl::MoveTo(point) | kurbo::PathEl::LineTo(point) = element {
                    let at = kurbo::Rect::from_points(*point, *point);
                    bounds = Some(bounds.map_or(at, |bounds| bounds.union(at)));
                }
            }
            if let Some(bounds) = bounds {
                return to_rect(bounds);
            }
        }

        to_rect(self.elements.bounding_box())
    }

    /// Whether `other` is this very path: the same elements, not equal ones.
    pub fn is_same(&self, other: &VelloPath) -> bool {
        Arc::ptr_eq(&self.elements, &other.elements) && self.fill_rule == other.fill_rule
    }

    /// Whether the fill of the path contains the point. Open figures are
    /// filled as if they were closed.
    pub fn contains(&self, point: Point) -> bool {
        let winding = closed_path(&self.elements).winding(to_kurbo_point(point));
        if self.fill_rule == FillRule::EvenOdd {
            winding & 1 != 0
        } else {
            winding != 0
        }
    }
}

/// The fill path of a geometry in relation to its stroke path.
///
/// Many geometries fill exactly what they stroke; keeping that fact explicit
/// (rather than comparing paths) lets combined and transformed geometries
/// reuse one path for both.
#[derive(Clone, Debug, Default)]
pub enum FillPath {
    /// The geometry has no fill.
    #[default]
    None,
    /// The fill path is the stroke path.
    SameAsStroke,
    /// The fill path differs from the stroke path.
    Separate(VelloPath),
}

/// A geometry of this backend: something with a stroke path and a fill path.
///
/// The members of the platform contract are implemented once, by the
/// functions of this module, on top of the two paths; every geometry type
/// forwards to them through [`impl_geometry_impl`].
pub trait GeometryImpl: IGeometryImpl {
    /// The caches shared by all geometry kinds.
    fn base(&self) -> &GeometryImplBase;

    /// The path that is stroked.
    fn stroke_path(&self) -> Option<VelloPath>;

    /// The fill of the geometry in relation to its stroke path.
    fn fill(&self) -> FillPath;

    /// The path that is filled.
    fn fill_path(&self) -> Option<VelloPath> {
        match self.fill() {
            FillPath::None => None,
            FillPath::SameAsStroke => self.stroke_path(),
            FillPath::Separate(path) => Some(path),
        }
    }

    /// Whether the fill path is the very same path as the stroke path.
    fn is_fill_same_as_stroke(&self) -> bool {
        matches!(self.fill(), FillPath::SameAsStroke) && self.stroke_path().is_some()
    }
}

/// State every geometry kind has: the stroke cache, the path measure and a
/// handle to the geometry itself.
#[derive(Default)]
pub struct GeometryImplBase {
    path_cache: Mutex<PathCache>,
    cached_path_measure: Mutex<Option<PathMeasure>>,
    weak_self: Mutex<Option<Weak<dyn GeometryImpl>>>,
}

/// A value of a geometry that the UI thread and the render thread both read:
/// readers take a copy under a lock; a copy of a path shares its elements.
#[derive(Default)]
pub struct Shared<T>(Mutex<T>);

impl<T: Clone> Shared<T> {
    /// Wraps `value`.
    pub fn new(value: T) -> Self {
        Self(Mutex::new(value))
    }

    /// A copy of the value.
    pub fn get(&self) -> T {
        self.0.lock().unwrap().clone()
    }

    /// Replaces the value.
    pub fn set(&self, value: T) {
        *self.0.lock().unwrap() = value;
    }
}

impl GeometryImplBase {
    /// Creates the state of a new geometry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Invalidates all caches. Call after the paths of the geometry changed.
    pub fn invalidate_caches(&self) {
        *self.path_cache.lock().unwrap() = PathCache::default();
        *self.cached_path_measure.lock().unwrap() = None;
    }

    /// The handle of the geometry this state belongs to.
    fn to_arc(&self) -> Arc<dyn GeometryImpl> {
        self.weak_self
            .lock()
            .unwrap()
            .as_ref()
            .and_then(Weak::upgrade)
            .expect("geometries of the backend are created behind a handle")
    }
}

/// Puts a new geometry behind a handle and lets it find that handle again
/// (a transformed geometry keeps its source alive).
pub(crate) fn register<T: GeometryImpl>(geometry: T) -> Arc<T> {
    let geometry = Arc::new(geometry);
    let weak: Weak<dyn GeometryImpl> = Arc::downgrade(&(geometry.clone() as Arc<dyn GeometryImpl>));
    *geometry.base().weak_self.lock().unwrap() = Some(weak);
    geometry
}

/// Recovers the backend geometry behind a platform geometry.
///
/// Returns `None` for geometries created by another backend.
pub fn try_get_geometry_impl(geometry: &dyn IGeometryImpl) -> Option<&dyn GeometryImpl> {
    use crate::ellipse_geometry_impl::EllipseGeometryImpl;
    use crate::geometry_group_impl::GeometryGroupImpl;
    use crate::line_geometry_impl::LineGeometryImpl;
    use crate::rectangle_geometry_impl::RectangleGeometryImpl;

    let any = geometry.as_any();

    if let Some(geometry) = any.downcast_ref::<StreamGeometryImpl>() {
        return Some(geometry);
    }
    if let Some(geometry) = any.downcast_ref::<TransformedGeometryImpl>() {
        return Some(geometry);
    }
    if let Some(geometry) = any.downcast_ref::<RectangleGeometryImpl>() {
        return Some(geometry);
    }
    if let Some(geometry) = any.downcast_ref::<EllipseGeometryImpl>() {
        return Some(geometry);
    }
    if let Some(geometry) = any.downcast_ref::<LineGeometryImpl>() {
        return Some(geometry);
    }
    if let Some(geometry) = any.downcast_ref::<GeometryGroupImpl>() {
        return Some(geometry);
    }
    if let Some(geometry) = any.downcast_ref::<CombinedGeometryImpl>() {
        return Some(geometry);
    }

    None
}

fn with_path_measure<R>(geometry: &dyn GeometryImpl, f: impl FnOnce(&PathMeasure) -> R) -> Option<R> {
    let stroke_path = geometry.stroke_path()?;
    let mut measure = geometry.base().cached_path_measure.lock().unwrap();
    let measure = measure.get_or_insert_with(|| PathMeasure::new(stroke_path.path()));
    Some(f(measure))
}

pub(crate) fn contour_length(geometry: &dyn GeometryImpl) -> f64 {
    with_path_measure(geometry, |measure| measure.length()).unwrap_or(0.0)
}

pub(crate) fn fill_contains(geometry: &dyn GeometryImpl, point: Point) -> bool {
    geometry.fill_path().is_some_and(|path| path.contains(point))
}

pub(crate) fn stroke_contains(geometry: &dyn GeometryImpl, pen: Option<&dyn IPen>, point: Point) -> bool {
    let mut cache = geometry.base().path_cache.lock().unwrap();
    cache.update_if_needed(geometry.stroke_path().as_ref(), pen);
    cache.expanded_path().is_some_and(|path| path.contains(point))
}

pub(crate) fn intersect(geometry: &dyn GeometryImpl, other: &dyn IGeometryImpl) -> Option<Arc<dyn IGeometryImpl>> {
    let other = try_get_geometry_impl(other)?;
    CombinedGeometryImpl::try_create(GeometryCombineMode::Intersect, geometry, other)
        .map(|geometry| geometry as Arc<dyn IGeometryImpl>)
}

pub(crate) fn get_render_bounds(geometry: &dyn GeometryImpl, pen: Option<&dyn IPen>) -> Rect {
    let mut cache = geometry.base().path_cache.lock().unwrap();
    cache.update_if_needed(geometry.stroke_path().as_ref(), pen);
    let mut bounds = cache.render_bounds();

    if let FillPath::Separate(fill_path) = geometry.fill() {
        bounds = bounds.union(fill_path.tight_bounds());
    }

    bounds
}

pub(crate) fn get_widened_geometry(geometry: &dyn GeometryImpl, pen: &dyn IPen) -> Arc<dyn IGeometryImpl> {
    if let Some(stroke_path) = geometry.stroke_path() {
        if let Some(path) = path_helper::create_stroked_path(stroke_path.path(), pen) {
            // The outline of a stroke is filled by the non-zero rule: where
            // the stroke crosses itself its figures overlap.
            return StreamGeometryImpl::from_paths(
                VelloPath::new(path, FillRule::NonZero),
                FillPath::SameAsStroke,
                None,
            );
        }
    }

    StreamGeometryImpl::from_paths(VelloPath::empty(), FillPath::None, None)
}

pub(crate) fn with_transform(geometry: &dyn GeometryImpl, transform: Matrix) -> Arc<dyn ITransformedGeometryImpl> {
    TransformedGeometryImpl::new(geometry.base().to_arc(), transform)
}

pub(crate) fn try_get_point_at_distance(geometry: &dyn GeometryImpl, distance: f64) -> Option<Point> {
    try_get_point_and_tangent_at_distance(geometry, distance).map(|(point, _)| point)
}

pub(crate) fn try_get_point_and_tangent_at_distance(
    geometry: &dyn GeometryImpl,
    distance: f64,
) -> Option<(Point, Point)> {
    let (point, tangent) = with_path_measure(geometry, |measure| measure.pos_tan(distance))??;
    Some((to_point(point), Point::new(tangent.x, tangent.y)))
}

pub(crate) fn try_get_segment(
    geometry: &dyn GeometryImpl,
    start_distance: f64,
    stop_distance: f64,
    start_on_begin_figure: bool,
) -> Option<Arc<dyn IGeometryImpl>> {
    let segment = with_path_measure(geometry, |measure| {
        measure.get_segment(start_distance, stop_distance, start_on_begin_figure)
    })??;

    Some(StreamGeometryImpl::from_paths(VelloPath::new(segment, FillRule::EvenOdd), FillPath::None, None))
}

pub(crate) fn get_fill_intersection_result(
    geometry: &dyn GeometryImpl,
    other: &dyn IGeometryImpl,
) -> IntersectionResult {
    let Some(other) = try_get_geometry_impl(other) else {
        return IntersectionResult::Empty;
    };

    hit_test_path(geometry.fill_path().as_ref(), other.fill_path().as_ref())
}

/// The relation of the areas two paths fill.
///
/// The Skia backend compares the regions of the two paths, which are the
/// pixels they cover; here the areas themselves are compared: they
/// intersect when their intersection has an area, and one contains the
/// other when nothing of the other is left outside it.
fn hit_test_path(path1: Option<&VelloPath>, path2: Option<&VelloPath>) -> IntersectionResult {
    use crate::combined_geometry_impl::{combine_paths, path_area};

    let (Some(path1), Some(path2)) = (path1, path2) else {
        return IntersectionResult::Empty;
    };

    /// Areas below this are the noise of the boolean operations.
    const MINIMUM_AREA: f64 = 1e-6;

    let has_area = |mode| {
        combine_paths(mode, path1, path2).is_some_and(|path| path_area(path.path()) > MINIMUM_AREA)
    };

    if has_area(GeometryCombineMode::Intersect) {
        let other_outside = combine_paths(GeometryCombineMode::Exclude, path2, path1)
            .is_some_and(|path| path_area(path.path()) > MINIMUM_AREA);
        if !other_outside {
            return IntersectionResult::FullyInside;
        }

        if !has_area(GeometryCombineMode::Exclude) {
            return IntersectionResult::FullyContains;
        }

        return IntersectionResult::Intersects;
    }

    IntersectionResult::Empty
}

/// The stroke of a geometry expanded with a pen, kept for hit testing and
/// render bounds until the pen or the stroke path changes.
#[derive(Default)]
struct PathCache {
    pen_hash: u64,
    path: Option<VelloPath>,
    /// The stroke path the cache was computed for.
    cached_for: Option<VelloPath>,
    render_bounds: Option<Rect>,
}

impl PathCache {
    fn render_bounds(&mut self) -> Rect {
        if let Some(bounds) = self.render_bounds {
            return bounds;
        }

        let bounds = match (&self.path, &self.cached_for) {
            (Some(path), _) => path.tight_bounds(),
            (None, Some(cached_for)) => cached_for.tight_bounds(),
            (None, None) => Rect::default(),
        };
        self.render_bounds = Some(bounds);
        bounds
    }

    fn expanded_path(&self) -> Option<&VelloPath> {
        self.path.as_ref()
    }

    fn update_if_needed(&mut self, stroke_path: Option<&VelloPath>, pen: Option<&dyn IPen>) {
        let pen_hash = pen_helper::get_hash_code(pen, false);

        let same_path = match (stroke_path, &self.cached_for) {
            (Some(stroke_path), Some(cached_for)) => stroke_path.is_same(cached_for),
            (None, None) => true,
            _ => false,
        };

        if pen_hash == self.pen_hash && same_path {
            // We are up to date.
            return;
        }

        self.render_bounds = None;
        self.cached_for = stroke_path.cloned();
        self.pen_hash = pen_hash;

        self.path = match (stroke_path, pen) {
            (Some(stroke_path), Some(pen)) => path_helper::create_stroked_path(stroke_path.path(), pen)
                .map(|path| VelloPath::new(path, FillRule::NonZero)),
            _ => None,
        };
    }
}

/// Implements the platform geometry contract for a [`GeometryImpl`] type by
/// forwarding to the shared implementation.
///
/// The second form adds overrides of the contract's default members (the
/// `as_*` views).
macro_rules! impl_geometry_impl {
    ($ty:ty) => {
        $crate::geometry_impl::impl_geometry_impl!($ty, {});
    };
    ($ty:ty, { $($extra:tt)* }) => {
        impl ferroui_base::platform::IGeometryImpl for $ty {
            fn bounds(&self) -> ferroui_base::Rect {
                self.geometry_bounds()
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            fn contour_length(&self) -> f64 {
                $crate::geometry_impl::contour_length(self)
            }

            fn get_render_bounds(&self, pen: Option<&dyn ferroui_base::media::IPen>) -> ferroui_base::Rect {
                $crate::geometry_impl::get_render_bounds(self, pen)
            }

            fn get_widened_geometry(
                &self,
                pen: &dyn ferroui_base::media::IPen,
            ) -> std::sync::Arc<dyn ferroui_base::platform::IGeometryImpl> {
                $crate::geometry_impl::get_widened_geometry(self, pen)
            }

            fn fill_contains(&self, point: ferroui_base::Point) -> bool {
                $crate::geometry_impl::fill_contains(self, point)
            }

            fn get_fill_intersection_result(
                &self,
                geometry: &dyn ferroui_base::platform::IGeometryImpl,
            ) -> ferroui_base::media::IntersectionResult {
                $crate::geometry_impl::get_fill_intersection_result(self, geometry)
            }

            fn intersect(
                &self,
                geometry: &dyn ferroui_base::platform::IGeometryImpl,
            ) -> Option<std::sync::Arc<dyn ferroui_base::platform::IGeometryImpl>> {
                $crate::geometry_impl::intersect(self, geometry)
            }

            fn stroke_contains(
                &self,
                pen: Option<&dyn ferroui_base::media::IPen>,
                point: ferroui_base::Point,
            ) -> bool {
                $crate::geometry_impl::stroke_contains(self, pen, point)
            }

            fn with_transform(
                &self,
                transform: ferroui_base::Matrix,
            ) -> std::sync::Arc<dyn ferroui_base::platform::ITransformedGeometryImpl> {
                $crate::geometry_impl::with_transform(self, transform)
            }

            fn try_get_point_at_distance(&self, distance: f64) -> Option<ferroui_base::Point> {
                $crate::geometry_impl::try_get_point_at_distance(self, distance)
            }

            fn try_get_point_and_tangent_at_distance(
                &self,
                distance: f64,
            ) -> Option<(ferroui_base::Point, ferroui_base::Point)> {
                $crate::geometry_impl::try_get_point_and_tangent_at_distance(self, distance)
            }

            fn try_get_segment(
                &self,
                start_distance: f64,
                stop_distance: f64,
                start_on_begin_figure: bool,
            ) -> Option<std::sync::Arc<dyn ferroui_base::platform::IGeometryImpl>> {
                $crate::geometry_impl::try_get_segment(self, start_distance, stop_distance, start_on_begin_figure)
            }

            $($extra)*
        }
    };
}

pub(crate) use impl_geometry_impl;
