use crate::geometry_impl::{impl_geometry_impl, register, FillPath, GeometryImpl, GeometryImplBase, Shared, VelloPath};
use crate::vello_extensions::{to_kurbo_point, PATH_TOLERANCE};
use ferroui_base::media::{FillRule, SweepDirection};
use ferroui_base::platform::{IGeometryContext, IStreamGeometryContextImpl, IStreamGeometryImpl};
use ferroui_base::{Point, Rect, Size};
use kurbo::{BezPath, SvgArc, Vec2};
use std::sync::Arc;

/// The paths of a stream geometry, shared with the contexts that define it.
struct StreamGeometryState {
    base: GeometryImplBase,
    bounds: Shared<Rect>,
    stroke_path: Shared<VelloPath>,
    fill: Shared<FillPath>,
}

/// A kurbo implementation of a stream geometry.
pub struct StreamGeometryImpl {
    state: Arc<StreamGeometryState>,
}

impl StreamGeometryImpl {
    /// Creates a geometry from a stroke path and a fill. `bounds` defaults to
    /// the tight bounds of the stroke path.
    pub fn from_paths(stroke: VelloPath, fill: FillPath, bounds: Option<Rect>) -> Arc<Self> {
        let bounds = bounds.unwrap_or_else(|| stroke.tight_bounds());

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
        Self::from_paths(VelloPath::empty(), FillPath::SameAsStroke, Some(Rect::default()))
    }

    fn geometry_bounds(&self) -> Rect {
        self.state.bounds.get()
    }
}

impl GeometryImpl for StreamGeometryImpl {
    fn base(&self) -> &GeometryImplBase {
        &self.state.base
    }

    fn stroke_path(&self) -> Option<VelloPath> {
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

/// A path under construction with the rule it will be filled by.
///
/// The commands are those of the path builder of Skia, with its rules for a
/// command that has no point to start from: it starts at the origin.
#[derive(Clone)]
struct PathBuilder {
    path: BezPath,
    fill_rule: FillRule,
}

impl PathBuilder {
    /// A builder of an empty path, filled by the non-zero rule.
    fn new() -> Self {
        Self { path: BezPath::new(), fill_rule: FillRule::NonZero }
    }

    fn from_path(path: &VelloPath) -> Self {
        Self { path: path.path().clone(), fill_rule: path.fill_rule() }
    }

    fn snapshot(&self) -> VelloPath {
        VelloPath::new(self.path.clone(), self.fill_rule)
    }

    /// The point the next command starts from.
    fn current_point(&mut self) -> kurbo::Point {
        // (A path of kurbo is "empty" until it has a segment: the elements
        // are what tells whether a figure was begun.)
        if self.path.elements().is_empty() {
            self.path.move_to((0.0, 0.0));
        }
        self.path.current_position().unwrap_or_default()
    }

    fn move_to(&mut self, point: kurbo::Point) {
        self.path.move_to(point);
    }

    fn line_to(&mut self, point: kurbo::Point) {
        self.current_point();
        self.path.line_to(point);
    }

    fn quad_to(&mut self, p1: kurbo::Point, p2: kurbo::Point) {
        self.current_point();
        self.path.quad_to(p1, p2);
    }

    fn cubic_to(&mut self, p1: kurbo::Point, p2: kurbo::Point, p3: kurbo::Point) {
        self.current_point();
        self.path.curve_to(p1, p2, p3);
    }

    fn close(&mut self) {
        if !self.path.elements().is_empty() {
            self.path.close_path();
        }
    }

    /// Adds an elliptical arc from the current point to `end`, with the
    /// endpoint parameters of an arc of SVG (which the contract describes):
    /// radii that are too small to reach the end point are scaled up, and
    /// an arc without a radius or without a length is a line to the end
    /// point. The arc is added as cubic Béziers.
    fn arc_to(&mut self, radii: Vec2, rotation_angle: f64, is_large_arc: bool, clockwise: bool, end: kurbo::Point) {
        let from = self.current_point();
        let arc = SvgArc {
            from,
            to: end,
            radii,
            x_rotation: rotation_angle.to_radians(),
            large_arc: is_large_arc,
            // The sweep flag of SVG: the arc is drawn in the direction of
            // growing angles, which is clockwise with the y axis down.
            sweep: clockwise,
        };

        match kurbo::Arc::from_svg_arc(&arc) {
            Some(arc) => {
                let mut curves = Vec::new();
                arc.to_cubic_beziers(PATH_TOLERANCE, |p1, p2, p3| curves.push((p1, p2, p3)));
                // The last Bézier ends at the end of the arc as it was
                // computed; the figure continues from the end point as it
                // was given.
                if let Some(last) = curves.last_mut() {
                    last.2 = end;
                }
                for (p1, p2, p3) in curves {
                    self.path.curve_to(p1, p2, p3);
                }
            }
            None => self.path.line_to(end),
        }
    }
}

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
/// The paths of a geometry are immutable, so the context builds on path
/// builders seeded from the geometry's current paths and writes the result
/// back when it is disposed (or dropped).
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
        let stroke = PathBuilder::from_path(&geometry_impl.stroke_path.get());
        let fill = match &geometry_impl.fill.get() {
            FillPath::None => FillBuilder::None,
            FillPath::SameAsStroke => FillBuilder::SameAsStroke,
            FillPath::Separate(path) => FillBuilder::Separate(PathBuilder::from_path(path)),
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
                self.fill = FillBuilder::Separate(self.stroke.clone());
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

        self.geometry_impl.bounds.set(stroke.tight_bounds());
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
        let clockwise = sweep_direction == SweepDirection::Clockwise;
        let radii = Vec2::new(size.width, size.height);
        let end = to_kurbo_point(point);

        if is_stroked {
            self.stroke.arc_to(radii, rotation_angle, is_large_arc, clockwise, end);
        } else {
            self.break_figure();
            self.stroke.move_to(end);
        }

        if self.duplicate() {
            self.fill().arc_to(radii, rotation_angle, is_large_arc, clockwise, end);
        }
    }

    fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
        if !is_filled {
            self.ensure_separate_fill_path();
        }

        self.is_filled = is_filled;
        self.start_point = start_point;
        self.is_figure_broken = false;
        self.stroke.move_to(to_kurbo_point(start_point));

        if self.duplicate() {
            self.fill().move_to(to_kurbo_point(start_point));
        }
    }

    fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool) {
        let (p1, p2, p3) =
            (to_kurbo_point(control_point1), to_kurbo_point(control_point2), to_kurbo_point(end_point));

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
        let (p1, p2) = (to_kurbo_point(control_point), to_kurbo_point(end_point));

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
        let point = to_kurbo_point(point);

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
                self.stroke.line_to(to_kurbo_point(self.start_point));
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
        self.fill().fill_rule = fill_rule;
    }

    fn dispose(&mut self) {
        self.commit();
    }
}

impl IStreamGeometryContextImpl for StreamContext {}
