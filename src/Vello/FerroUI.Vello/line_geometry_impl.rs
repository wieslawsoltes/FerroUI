use crate::geometry_impl::{impl_geometry_impl, register, FillPath, GeometryImpl, GeometryImplBase, VelloPath};
use crate::vello_extensions::to_kurbo_point;
use ferroui_base::media::FillRule;
use ferroui_base::{Point, Rect};
use kurbo::BezPath;
use std::sync::Arc;

/// A kurbo implementation of a line geometry.
pub struct LineGeometryImpl {
    base: GeometryImplBase,
    bounds: Rect,
    stroke_path: VelloPath,
}

impl LineGeometryImpl {
    /// Creates the geometry of the line from `p1` to `p2`.
    pub fn new(p1: Point, p2: Point) -> Arc<Self> {
        let mut path = BezPath::new();
        path.move_to(to_kurbo_point(p1));
        path.line_to(to_kurbo_point(p2));

        let bounds = Rect::from_points(
            Point::new(p1.x.min(p2.x), p1.y.min(p2.y)),
            Point::new(p1.x.max(p2.x), p1.y.max(p2.y)),
        );

        register(Self {
            base: GeometryImplBase::new(),
            bounds,
            stroke_path: VelloPath::new(path, FillRule::NonZero),
        })
    }

    fn geometry_bounds(&self) -> Rect {
        self.bounds
    }
}

impl GeometryImpl for LineGeometryImpl {
    fn base(&self) -> &GeometryImplBase {
        &self.base
    }

    fn stroke_path(&self) -> Option<VelloPath> {
        Some(self.stroke_path.clone())
    }

    fn fill(&self) -> FillPath {
        FillPath::None
    }
}

impl_geometry_impl!(LineGeometryImpl);
