use crate::geometry_impl::{impl_geometry_impl, register, FillPath, GeometryImpl, GeometryImplBase, Shared};
use crate::skia_sharp_extensions::{to_rect, to_sk_point};
use ferroui_base::media::{FillRule, SweepDirection};
use ferroui_base::platform::{IGeometryContext, IStreamGeometryContextImpl, IStreamGeometryImpl};
use ferroui_base::{Point, Rect, Size};
use skia_safe::path_builder::ArcSize;
use skia_safe::{Path, PathBuilder, PathDirection, PathFillType};
use std::sync::Arc;

/// The paths of a stream geometry, shared with the contexts that define it.
struct StreamGeometryState {
    base: GeometryImplBase,
    bounds: Shared<Rect>,
    stroke_path: Shared<Path>,
    fill: Shared<FillPath>,
}

/// A Skia implementation of a stream geometry.
pub struct StreamGeometryImpl {
    state: Arc<StreamGeometryState>,
}

impl StreamGeometryImpl {
    /// Creates a geometry from a stroke path and a fill. `bounds` defaults to
    /// the tight bounds of the stroke path.
    pub fn from_paths(stroke: Path, fill: FillPath, bounds: Option<Rect>) -> Arc<Self> {
        let bounds = bounds.unwrap_or_else(|| to_rect(stroke.compute_tight_bounds()));

        register(Self {
            state: Arc::new(StreamGeometryState {
                base: GeometryImplBase::new(),
                bounds: Shared::new(bounds),
                stroke_path: Shared::new(stroke),
                fill: Shared::new(fill),
            }),
        })
    }

    /// Creates an empty geometry.
    pub fn new() -> Arc<Self> {
        Self::from_paths(Self::create_empty_path(), FillPath::SameAsStroke, Some(Rect::default()))
    }

    fn create_empty_path() -> Path {
        Path::new_with_fill_type(PathFillType::EvenOdd)
    }

    fn geometry_bounds(&self) -> Rect {
        self.state.bounds.get()
    }
}

impl GeometryImpl for StreamGeometryImpl {
    fn base(&self) -> &GeometryImplBase {
        &self.state.base
    }

    fn stroke_path(&self) -> Option<Path> {
        Some(self.state.stroke_path.get())
    }

    fn fill(&self) -> FillPath {
        self.state.fill.get()
    }
}

impl IStreamGeometryImpl for StreamGeometryImpl {
    fn clone_geometry(&self) -> Arc<dyn IStreamGeometryImpl> {
        let stroke = self.state.stroke_path.get();
        let fill = self.state.fill.get();

        StreamGeometryImpl::from_paths(stroke, fill, Some(self.geometry_bounds()))
    }

    fn open(&self) -> Box<dyn IStreamGeometryContextImpl> {
        Box::new(StreamContext::new(self.state.clone()))
    }
}

impl_geometry_impl!(StreamGeometryImpl, {
    fn as_stream_geometry(&self) -> Option<&dyn IStreamGeometryImpl> {
        Some(self)
    }
});

/// The fill being built by a stream context.
enum FillBuilder {
    /// No fill path exists yet.
    None,
    /// The fill is the stroke.
    SameAsStroke,
    /// The fill is built separately from the stroke.
    Separate(PathBuilder),
}

/// A simple implementation of a stream geometry context.
///
/// Skia paths are immutable, so the context builds on path builders seeded
/// from the geometry's current paths and writes the result back when it is
/// disposed (or dropped).
struct StreamContext {
    geometry_impl: Arc<StreamGeometryState>,
    stroke: PathBuilder,
    fill: FillBuilder,
    is_filled: bool,
    start_point: Point,
    is_figure_broken: bool,
    is_disposed: bool,
}

impl StreamContext {
    fn new(geometry_impl: Arc<StreamGeometryState>) -> Self {
        let stroke = PathBuilder::new_path(&geometry_impl.stroke_path.get());
        let fill = match &geometry_impl.fill.get() {
            FillPath::None => FillBuilder::None,
            FillPath::SameAsStroke => FillBuilder::SameAsStroke,
            FillPath::Separate(path) => FillBuilder::Separate(PathBuilder::new_path(path)),
        };

        Self {
            geometry_impl,
            stroke,
            fill,
            is_filled: false,
            start_point: Point::default(),
            is_figure_broken: false,
            is_disposed: false,
        }
    }

    /// The fill builder, created on first use.
    fn fill(&mut self) -> &mut PathBuilder {
        if matches!(self.fill, FillBuilder::None) {
            self.fill = FillBuilder::Separate(PathBuilder::new());
        }

        match &mut self.fill {
            FillBuilder::SameAsStroke => &mut self.stroke,
            FillBuilder::Separate(fill) => fill,
            FillBuilder::None => unreachable!("the fill builder was just created"),
        }
    }

    /// Whether commands have to be repeated on a separate fill path.
    fn duplicate(&self) -> bool {
        self.is_filled && !matches!(self.fill, FillBuilder::SameAsStroke)
    }

    fn ensure_separate_fill_path(&mut self) {
        match self.fill {
            FillBuilder::SameAsStroke => {
                self.fill = FillBuilder::Separate(PathBuilder::new_path(&self.stroke.snapshot()));
            }
            FillBuilder::None => {
                self.fill = FillBuilder::Separate(PathBuilder::new());
            }
            FillBuilder::Separate(_) => {}
        }
    }

    fn break_figure(&mut self) {
        if !self.is_figure_broken {
            self.is_figure_broken = true;
            self.ensure_separate_fill_path();
        }
    }

    /// Writes the built paths back to the geometry.
    fn commit(&mut self) {
        if self.is_disposed {
            return;
        }
        self.is_disposed = true;

        let stroke = self.stroke.snapshot();
        let fill = match &self.fill {
            FillBuilder::None => FillPath::None,
            FillBuilder::SameAsStroke => FillPath::SameAsStroke,
            FillBuilder::Separate(fill) => FillPath::Separate(fill.snapshot()),
        };

        self.geometry_impl.bounds.set(to_rect(stroke.compute_tight_bounds()));
        self.geometry_impl.stroke_path.set(stroke);
        self.geometry_impl.fill.set(fill);
        self.geometry_impl.base.invalidate_caches();
    }
}

impl Drop for StreamContext {
    fn drop(&mut self) {
        self.commit();
    }
}

impl IGeometryContext for StreamContext {
    fn arc_to(
        &mut self,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
        is_stroked: bool,
    ) {
        let arc = if is_large_arc { ArcSize::Large } else { ArcSize::Small };
        let sweep =
            if sweep_direction == SweepDirection::Clockwise { PathDirection::CW } else { PathDirection::CCW };
        let radii = (size.width as f32, size.height as f32);
        let end = to_sk_point(point);

        if is_stroked {
            self.stroke.arc_to_radius(radii, rotation_angle as f32, arc, sweep, end);
        } else {
            self.break_figure();
            self.stroke.move_to(end);
        }

        if self.duplicate() {
            self.fill().arc_to_radius(radii, rotation_angle as f32, arc, sweep, end);
        }
    }

    fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
        if !is_filled {
            self.ensure_separate_fill_path();
        }

        self.is_filled = is_filled;
        self.start_point = start_point;
        self.is_figure_broken = false;
        self.stroke.move_to(to_sk_point(start_point));

        if self.duplicate() {
            self.fill().move_to(to_sk_point(start_point));
        }
    }

    fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool) {
        let (p1, p2, p3) = (to_sk_point(control_point1), to_sk_point(control_point2), to_sk_point(end_point));

        if is_stroked {
            self.stroke.cubic_to(p1, p2, p3);
        } else {
            self.break_figure();
            self.stroke.move_to(p3);
        }

        if self.duplicate() {
            self.fill().cubic_to(p1, p2, p3);
        }
    }

    fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, is_stroked: bool) {
        let (p1, p2) = (to_sk_point(control_point), to_sk_point(end_point));

        if is_stroked {
            self.stroke.quad_to(p1, p2);
        } else {
            self.break_figure();
            self.stroke.move_to(p2);
        }

        if self.duplicate() {
            self.fill().quad_to(p1, p2);
        }
    }

    fn line_to(&mut self, point: Point, is_stroked: bool) {
        let point = to_sk_point(point);

        if is_stroked {
            self.stroke.line_to(point);
        } else {
            self.break_figure();
            self.stroke.move_to(point);
        }

        if self.duplicate() {
            self.fill().line_to(point);
        }
    }

    fn end_figure(&mut self, is_closed: bool) {
        if is_closed {
            if self.is_figure_broken {
                self.stroke.line_to(to_sk_point(self.start_point));
                self.is_figure_broken = false;
            } else {
                self.stroke.close();
            }

            if self.duplicate() {
                self.fill().close();
            }
        }
    }

    fn set_fill_rule(&mut self, fill_rule: FillRule) {
        let fill_type = if fill_rule == FillRule::EvenOdd { PathFillType::EvenOdd } else { PathFillType::Winding };
        self.fill().set_fill_type(fill_type);
    }

    fn dispose(&mut self) {
        self.commit();
    }
}

impl IStreamGeometryContextImpl for StreamContext {}
