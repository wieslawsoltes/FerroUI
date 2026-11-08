use crate::{
    ffi::{self, Command, Owned},
    path_measure::PathMeasure,
};
use ferroui_base::{
    media::{
        FillRule, GeometryCombineMode, GeometryPath, GeometryPathData, GeometryPathSegment, IPen,
        IntersectionResult, PenLineCap, PenLineJoin, SweepDirection,
    },
    platform::{IGeometryContext, IGeometryImpl, IStreamGeometryBuilder, ITransformedGeometryImpl},
    Matrix, Point, Rect, Size,
};
use kurbo::{Affine, BezPath, PathEl, Shape, SvgArc, Vec2};
use std::{any::Any, sync::Arc};
fn point(p: Point) -> kurbo::Point {
    kurbo::Point::new(p.x, p.y)
}
fn rect(r: kurbo::Rect) -> Rect {
    Rect::new(r.x0, r.y0, r.width(), r.height())
}
pub fn affine(m: Matrix) -> Affine {
    Affine::new([m.m11, m.m12, m.m21, m.m22, m.m31, m.m32])
}
pub fn native(path: &BezPath) -> Owned {
    let commands: Vec<Command> = path
        .elements()
        .iter()
        .map(|p| match *p {
            PathEl::MoveTo(p) => Command {
                kind: 0,
                p: [p.x, p.y, 0., 0., 0., 0.],
            },
            PathEl::LineTo(p) => Command {
                kind: 1,
                p: [p.x, p.y, 0., 0., 0., 0.],
            },
            PathEl::QuadTo(a, b) => Command {
                kind: 2,
                p: [a.x, a.y, b.x, b.y, 0., 0.],
            },
            PathEl::CurveTo(a, b, c) => Command {
                kind: 3,
                p: [a.x, a.y, b.x, b.y, c.x, c.y],
            },
            PathEl::ClosePath => Command {
                kind: 4,
                ..Default::default()
            },
        })
        .collect();
    unsafe { Owned::new(ffi::q_path(commands.as_ptr(), commands.len())) }
}
pub fn from_native(p: ffi::Ptr) -> BezPath {
    let mut path = BezPath::new();
    extern "C" fn emit(info: ffi::Ptr, c: Command) {
        unsafe {
            let path = &mut *(info as *mut BezPath);
            let p = c.p;
            match c.kind {
                0 => path.move_to((p[0], p[1])),
                1 => path.line_to((p[0], p[1])),
                2 => path.quad_to((p[0], p[1]), (p[2], p[3])),
                3 => path.curve_to((p[0], p[1]), (p[2], p[3]), (p[4], p[5])),
                _ => path.close_path(),
            }
        }
    }
    if !p.is_null() {
        unsafe { ffi::q_path_elements(p, &mut path as *mut _ as ffi::Ptr, emit) }
    }
    path
}
#[derive(Clone)]
pub struct Geometry {
    pub stroke: BezPath,
    pub fill: Option<BezPath>,
    pub rule: FillRule,
    source: Option<Arc<dyn IGeometryImpl>>,
    transform: Matrix,
}
impl Geometry {
    pub fn new(path: BezPath, rule: FillRule) -> Arc<Self> {
        Arc::new(Self {
            fill: Some(path.clone()),
            stroke: path,
            rule,
            source: None,
            transform: Matrix::IDENTITY,
        })
    }
    pub fn paths(stroke: BezPath, fill: Option<BezPath>, rule: FillRule) -> Arc<Self> {
        Arc::new(Self {
            stroke,
            fill,
            rule,
            source: None,
            transform: Matrix::IDENTITY,
        })
    }
    pub fn rectangle(r: Rect) -> Arc<Self> {
        Self::new(
            kurbo::Rect::new(r.x, r.y, r.right(), r.bottom()).to_path(0.001),
            FillRule::NonZero,
        )
    }
    pub fn ellipse(r: Rect) -> Arc<Self> {
        Self::new(
            kurbo::Ellipse::from_rect(kurbo::Rect::new(r.x, r.y, r.right(), r.bottom()))
                .to_path(0.001),
            FillRule::NonZero,
        )
    }
    pub fn combine(
        a: &dyn IGeometryImpl,
        b: &dyn IGeometryImpl,
        mode: GeometryCombineMode,
    ) -> Arc<Self> {
        let a = a
            .as_any()
            .downcast_ref::<Self>()
            .expect("Quartz geometry required");
        let b = b
            .as_any()
            .downcast_ref::<Self>()
            .expect("Quartz geometry required");
        let empty = BezPath::new();
        let na = native(a.fill.as_ref().unwrap_or(&empty));
        let nb = native(b.fill.as_ref().unwrap_or(&empty));
        let op = match mode {
            GeometryCombineMode::Union => 0,
            GeometryCombineMode::Intersect => 1,
            GeometryCombineMode::Xor => 2,
            GeometryCombineMode::Exclude => 3,
        };
        unsafe {
            let p = ffi::q_combine(
                na.0,
                nb.0,
                op,
                i32::from(a.rule == FillRule::EvenOdd),
                i32::from(b.rule == FillRule::EvenOdd),
            );
            assert!(!p.is_null(), "Core Graphics geometry combination failed");
            let p = Owned::new(p);
            Self::new(from_native(p.0), FillRule::NonZero)
        }
    }
    pub fn stroked(path: &BezPath, pen: &dyn IPen) -> BezPath {
        let p = native(path);
        let cap = match pen.line_cap() {
            PenLineCap::Flat => 0,
            PenLineCap::Round => 1,
            PenLineCap::Square => 2,
        };
        let join = match pen.line_join() {
            PenLineJoin::Miter => 0,
            PenLineJoin::Round => 1,
            PenLineJoin::Bevel => 2,
        };
        let dash = pen.dash_style();
        let values: Vec<f64> = dash
            .as_ref()
            .map(|d| {
                d.dashes()
                    .unwrap_or_default()
                    .iter()
                    .map(|v| v * pen.thickness())
                    .collect()
            })
            .unwrap_or_default();
        let offset = dash.as_ref().map_or(0., |d| d.offset() * pen.thickness());
        unsafe {
            let p = Owned::new(ffi::q_stroke(
                p.0,
                pen.thickness(),
                cap,
                join,
                pen.miter_limit(),
                values.as_ptr(),
                values.len(),
                offset,
            ));
            from_native(p.0)
        }
    }
}
impl IGeometryImpl for Geometry {
    fn bounds(&self) -> Rect {
        match self.fill.as_ref().filter(|p| !p.is_empty()) {
            Some(fill) if self.stroke.is_empty() => rect(fill.bounding_box()),
            Some(fill) => rect(self.stroke.bounding_box().union(fill.bounding_box())),
            None => rect(self.stroke.bounding_box()),
        }
    }
    fn contour_length(&self) -> f64 {
        PathMeasure::new(&self.stroke).length()
    }
    fn get_render_bounds(&self, pen: Option<&dyn IPen>) -> Rect {
        pen.map_or_else(
            || self.bounds(),
            |p| {
                let stroke = Self::stroked(&self.stroke, p);
                match self.fill.as_ref().filter(|p| !p.is_empty()) {
                    Some(fill) if stroke.is_empty() => rect(fill.bounding_box()),
                    Some(fill) => rect(stroke.bounding_box().union(fill.bounding_box())),
                    None => rect(stroke.bounding_box()),
                }
            },
        )
    }
    fn get_widened_geometry(&self, pen: &dyn IPen) -> Arc<dyn IGeometryImpl> {
        Self::new(Self::stroked(&self.stroke, pen), FillRule::NonZero)
    }
    fn fill_contains(&self, p: Point) -> bool {
        self.fill.as_ref().is_some_and(|path| {
            let n = path.winding(point(p));
            if self.rule == FillRule::EvenOdd {
                n % 2 != 0
            } else {
                n != 0
            }
        })
    }
    fn stroke_contains(&self, pen: Option<&dyn IPen>, p: Point) -> bool {
        pen.is_some_and(|pen| Self::stroked(&self.stroke, pen).winding(point(p)) != 0)
    }
    fn get_fill_intersection_result(&self, g: &dyn IGeometryImpl) -> IntersectionResult {
        if !g.as_any().is::<Self>() {
            return IntersectionResult::Empty;
        }
        let combined = Self::combine(self, g, GeometryCombineMode::Intersect);
        let empty = |g: &Self| g.fill.as_ref().is_none_or(|p| p.is_empty());
        if empty(&combined) {
            return IntersectionResult::Empty;
        }
        // Match the established Ferro/Skia orientation: FullyInside means
        // the argument lies inside the receiver. Boolean differences preserve
        // holes and disjoint contours; signed areas can cancel and cannot
        // establish whether a filled path is empty.
        if empty(&Self::combine(g, self, GeometryCombineMode::Exclude)) {
            return IntersectionResult::FullyInside;
        }
        if empty(&Self::combine(self, g, GeometryCombineMode::Exclude)) {
            return IntersectionResult::FullyContains;
        }
        IntersectionResult::Intersects
    }
    fn intersect(&self, g: &dyn IGeometryImpl) -> Option<Arc<dyn IGeometryImpl>> {
        g.as_any().is::<Self>().then(|| {
            Self::combine(self, g, GeometryCombineMode::Intersect) as Arc<dyn IGeometryImpl>
        })
    }
    fn with_transform(&self, m: Matrix) -> Arc<dyn ITransformedGeometryImpl> {
        Arc::new(Self {
            stroke: affine(m) * &self.stroke,
            fill: self.fill.as_ref().map(|p| affine(m) * p),
            rule: self.rule,
            source: Some(Arc::new(self.clone())),
            transform: m,
        })
    }
    fn try_get_point_at_distance(&self, d: f64) -> Option<Point> {
        self.try_get_point_and_tangent_at_distance(d).map(|p| p.0)
    }
    fn try_get_point_and_tangent_at_distance(&self, d: f64) -> Option<(Point, Point)> {
        PathMeasure::new(&self.stroke)
            .pos_tan(d)
            .map(|(p, v)| (Point::new(p.x, p.y), Point::new(v.x, v.y)))
    }
    fn try_get_segment(&self, start: f64, end: f64, begin: bool) -> Option<Arc<dyn IGeometryImpl>> {
        PathMeasure::new(&self.stroke)
            .get_segment(start, end, begin)
            .map(|p| Self::paths(p, None, self.rule) as Arc<dyn IGeometryImpl>)
    }
    fn try_get_path_data(&self) -> Option<GeometryPathData> {
        let export = |p: &BezPath| GeometryPath {
            fill_rule: self.rule,
            segments: p
                .elements()
                .iter()
                .map(|e| match *e {
                    PathEl::MoveTo(p) => GeometryPathSegment::MoveTo(Point::new(p.x, p.y)),
                    PathEl::LineTo(p) => GeometryPathSegment::LineTo(Point::new(p.x, p.y)),
                    PathEl::QuadTo(a, p) => GeometryPathSegment::QuadraticTo {
                        control: Point::new(a.x, a.y),
                        point: Point::new(p.x, p.y),
                    },
                    PathEl::CurveTo(a, b, p) => GeometryPathSegment::CubicTo {
                        control1: Point::new(a.x, a.y),
                        control2: Point::new(b.x, b.y),
                        point: Point::new(p.x, p.y),
                    },
                    PathEl::ClosePath => GeometryPathSegment::Close,
                })
                .collect(),
        };
        Some(GeometryPathData {
            fill: self.fill.as_ref().map(export),
            stroke: Some(export(&self.stroke)),
        })
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_transformed_geometry(&self) -> Option<&dyn ITransformedGeometryImpl> {
        self.source
            .as_ref()
            .map(|_| self as &dyn ITransformedGeometryImpl)
    }
}
impl ITransformedGeometryImpl for Geometry {
    fn source_geometry(&self) -> Arc<dyn IGeometryImpl> {
        self.source
            .clone()
            .unwrap_or_else(|| Arc::new(self.clone()))
    }
    fn transform(&self) -> Matrix {
        self.transform
    }
}
pub struct Builder {
    stroke: BezPath,
    fill: BezPath,
    rule: FillRule,
    filled: bool,
    start: kurbo::Point,
    broken: bool,
    built: bool,
}
impl Builder {
    pub fn new(source: Option<&dyn IGeometryImpl>) -> Self {
        let g = source.map(|s| {
            s.as_any()
                .downcast_ref::<Geometry>()
                .expect("Quartz geometry required")
        });
        Self {
            stroke: g.map_or_else(BezPath::new, |g| g.stroke.clone()),
            fill: g.and_then(|g| g.fill.clone()).unwrap_or_default(),
            rule: g.map_or(FillRule::EvenOdd, |g| g.rule),
            filled: true,
            start: kurbo::Point::ZERO,
            broken: false,
            built: false,
        }
    }
    fn command(&mut self, e: PathEl, stroked: bool) {
        if stroked {
            self.stroke.push(e)
        } else {
            self.broken = true;
            let p = match e {
                PathEl::LineTo(p) | PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => p,
                _ => self.start,
            };
            self.stroke.move_to(p)
        }
        if self.filled {
            self.fill.push(e)
        }
    }
}
impl IStreamGeometryBuilder for Builder {
    fn to_geometry(&mut self) -> Arc<dyn IGeometryImpl> {
        assert!(!self.built, "The geometry has already been built");
        self.built = true;
        Geometry::paths(self.stroke.clone(), Some(self.fill.clone()), self.rule)
    }
}
impl IGeometryContext for Builder {
    fn begin_figure(&mut self, p: Point, filled: bool) {
        self.filled = filled;
        self.start = point(p);
        self.broken = false;
        self.stroke.move_to(self.start);
        if filled {
            self.fill.move_to(self.start)
        }
    }
    fn end_figure(&mut self, closed: bool) {
        if closed {
            if self.broken {
                self.stroke.line_to(self.start);
            } else {
                self.stroke.close_path();
            }
            if self.filled {
                self.fill.close_path();
            }
        }
    }
    fn set_fill_rule(&mut self, r: FillRule) {
        self.rule = r
    }
    fn line_to(&mut self, p: Point, s: bool) {
        self.command(PathEl::LineTo(point(p)), s)
    }
    fn cubic_bezier_to(&mut self, a: Point, b: Point, p: Point, s: bool) {
        self.command(PathEl::CurveTo(point(a), point(b), point(p)), s)
    }
    fn quadratic_bezier_to(&mut self, a: Point, p: Point, s: bool) {
        self.command(PathEl::QuadTo(point(a), point(p)), s)
    }
    fn arc_to(
        &mut self,
        p: Point,
        size: Size,
        rotation: f64,
        large: bool,
        sweep: SweepDirection,
        s: bool,
    ) {
        let arc = SvgArc {
            from: self.stroke.current_position().unwrap_or_default(),
            to: point(p),
            radii: Vec2::new(size.width.abs(), size.height.abs()),
            x_rotation: rotation.to_radians(),
            large_arc: large,
            sweep: sweep == SweepDirection::Clockwise,
        };
        if let Some(arc) = kurbo::Arc::from_svg_arc(&arc) {
            arc.to_cubic_beziers(0.001, |a, b, c| self.command(PathEl::CurveTo(a, b, c), s));
        } else {
            self.line_to(p, s)
        }
    }
    fn dispose(&mut self) {}
}
