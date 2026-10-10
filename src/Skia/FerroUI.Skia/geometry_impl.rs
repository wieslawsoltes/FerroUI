use crate::combined_geometry_impl::CombinedGeometryImpl;
use crate::helpers::{pen_helper, sk_path_helper};
use crate::skia_sharp_extensions::{to_rect, to_sk_point};
use crate::stream_geometry_impl::StreamGeometryImpl;
use crate::transformed_geometry_impl::TransformedGeometryImpl;
use ferroui_base::media::{GeometryCombineMode, IPen, IntersectionResult};
use ferroui_base::platform::{IGeometryImpl, ITransformedGeometryImpl};
use ferroui_base::{Matrix, Point, Rect};
use skia_safe::{Path, PathBuilder, PathMeasure, Region, RoundOut};
use std::sync::{Arc, Mutex, Weak};

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
    Separate(Path),
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
    fn stroke_path(&self) -> Option<Path>;

    /// The fill of the geometry in relation to its stroke path.
    fn fill(&self) -> FillPath;

    /// The path that is filled.
    fn fill_path(&self) -> Option<Path> {
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
    cached_path_measure: Mutex<Option<SendPathMeasure>>,
    weak_self: Mutex<Option<Weak<dyn GeometryImpl>>>,
}

/// A value of a geometry that the UI thread and the render thread both read.
///
/// The values of Skia held this way (paths) can be sent to another thread
/// but not shared by reference, so readers take a copy under a lock; a copy
/// of a path shares its storage.
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

/// The path measure cached by a geometry.
struct SendPathMeasure(PathMeasure);

// SAFETY: a path measure owns the contours it iterates (they are counted
// with atomic reference counts) and has no affinity to the thread that
// created it; the cache only hands it out under its lock.
unsafe impl Send for SendPathMeasure {}

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

fn with_path_measure<R>(geometry: &dyn GeometryImpl, f: impl FnOnce(&mut PathMeasure) -> R) -> Option<R> {
    let stroke_path = geometry.stroke_path()?;
    let mut measure = geometry.base().cached_path_measure.lock().unwrap();
    let measure = measure.get_or_insert_with(|| SendPathMeasure(PathMeasure::new(&stroke_path, false, None)));
    Some(f(&mut measure.0))
}

pub(crate) fn contour_length(geometry: &dyn GeometryImpl) -> f64 {
    with_path_measure(geometry, |measure| measure.length() as f64).unwrap_or(0.0)
}

pub(crate) fn fill_contains(geometry: &dyn GeometryImpl, point: Point) -> bool {
    path_contains_core(geometry.fill_path().as_ref(), point)
}

pub(crate) fn stroke_contains(geometry: &dyn GeometryImpl, pen: Option<&dyn IPen>, point: Point) -> bool {
    let mut cache = geometry.base().path_cache.lock().unwrap();
    cache.update_if_needed(geometry.stroke_path().as_ref(), pen);
    path_contains_core(cache.expanded_path(), point)
}

fn path_contains_core(path: Option<&Path>, point: Point) -> bool {
    path.is_some_and(|path| path.contains(to_sk_point(point)))
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
        bounds = bounds.union(to_rect(crate::skia_sharp_extensions::tight_bounds(&fill_path)));
    }

    bounds
}

pub(crate) fn get_widened_geometry(geometry: &dyn GeometryImpl, pen: &dyn IPen) -> Arc<dyn IGeometryImpl> {
    if let Some(stroke_path) = geometry.stroke_path() {
        if let Some(path) = sk_path_helper::create_stroked_path(&stroke_path, pen) {
            // The path returned by Skia here does not have closed figures.
            // Fix that by re-creating it closed.
            let closed = sk_path_helper::create_closed_path(&path);
            return StreamGeometryImpl::from_paths(closed, FillPath::SameAsStroke, None);
        }
    }

    StreamGeometryImpl::from_paths(Path::new(), FillPath::None, None)
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
    let (point, tangent) = with_path_measure(geometry, |measure| measure.pos_tan(distance as f32))??;
    Some((Point::new(point.x as f64, point.y as f64), Point::new(tangent.x as f64, tangent.y as f64)))
}

pub(crate) fn try_get_segment(
    geometry: &dyn GeometryImpl,
    start_distance: f64,
    stop_distance: f64,
    start_on_begin_figure: bool,
) -> Option<Arc<dyn IGeometryImpl>> {
    let mut segment = PathBuilder::new();
    let res = with_path_measure(geometry, |measure| {
        measure.get_segment(start_distance as f32, stop_distance as f32, &mut segment, start_on_begin_figure)
    })?;

    if res {
        Some(StreamGeometryImpl::from_paths(segment.detach(), FillPath::None, None))
    } else {
        None
    }
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

fn region_from_path(path: &Path) -> Region {
    let mut clip = Region::new();
    let rect: skia_safe::IRect = path.bounds().round_out();
    if !rect.is_empty() {
        clip.set_rect(rect);
    }

    let mut region = Region::new();
    region.set_path(path, &clip);
    region
}

fn hit_test_path(path1: Option<&Path>, path2: Option<&Path>) -> IntersectionResult {
    let (Some(path1), Some(path2)) = (path1, path2) else {
        return IntersectionResult::Empty;
    };

    let region = region_from_path(path1);
    let other_region = region_from_path(path2);

    if region.intersects_region(&other_region) {
        if region.contains_region(&other_region) {
            return IntersectionResult::FullyInside;
        }

        if other_region.contains_region(&region) {
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
    path: Option<Path>,
    /// Generation of the stroke path the cache was computed for.
    cached_for: Option<(u32, Path)>,
    render_bounds: Option<Rect>,
}

impl PathCache {
    fn render_bounds(&mut self) -> Rect {
        if let Some(bounds) = self.render_bounds {
            return bounds;
        }

        let bounds = match (&self.path, &self.cached_for) {
            (Some(path), _) => to_rect(crate::skia_sharp_extensions::tight_bounds(&path)),
            (None, Some((_, cached_for))) => to_rect(crate::skia_sharp_extensions::tight_bounds(&cached_for)),
            (None, None) => Rect::default(),
        };
        self.render_bounds = Some(bounds);
        bounds
    }

    fn expanded_path(&self) -> Option<&Path> {
        self.path.as_ref()
    }

    fn update_if_needed(&mut self, stroke_path: Option<&Path>, pen: Option<&dyn IPen>) {
        let pen_hash = pen_helper::get_hash_code(pen, false);
        let stroke_generation = stroke_path.map(Path::generation_id);

        if pen_hash == self.pen_hash && stroke_generation == self.cached_for.as_ref().map(|(generation, _)| *generation)
        {
            // We are up to date.
            return;
        }

        self.render_bounds = None;
        self.cached_for = stroke_path.map(|path| (path.generation_id(), path.clone()));
        self.pen_hash = pen_hash;

        self.path = match (stroke_path, pen) {
            (Some(stroke_path), Some(pen)) => sk_path_helper::create_stroked_path(stroke_path, pen),
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
