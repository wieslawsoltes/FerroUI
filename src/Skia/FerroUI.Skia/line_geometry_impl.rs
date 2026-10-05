use crate::geometry_impl::{impl_geometry_impl, register, FillPath, GeometryImpl, GeometryImplBase};
use crate::skia_sharp_extensions::to_sk_point;
use ferroui_base::{Point, Rect};
use skia_safe::{Path, PathBuilder};
use std::rc::Rc;

/// A Skia implementation of a line geometry.
pub struct LineGeometryImpl {
    base: GeometryImplBase,
    bounds: Rect,
    stroke_path: Path,
}

impl LineGeometryImpl {
    /// Creates the geometry of the line from `p1` to `p2`.
    pub fn new(p1: Point, p2: Point) -> Rc<Self> {
        let mut path = PathBuilder::new();
        path.move_to(to_sk_point(p1));
        path.line_to(to_sk_point(p2));

        let bounds = Rect::from_points(
            Point::new(p1.x.min(p2.x), p1.y.min(p2.y)),
            Point::new(p1.x.max(p2.x), p1.y.max(p2.y)),
        );

        register(Self { base: GeometryImplBase::new(), bounds, stroke_path: path.detach() })
    }

    fn geometry_bounds(&self) -> Rect {
        self.bounds
    }
}

impl GeometryImpl for LineGeometryImpl {
    fn base(&self) -> &GeometryImplBase {
        &self.base
    }

    fn stroke_path(&self) -> Option<Path> {
        Some(self.stroke_path.clone())
    }

    fn fill(&self) -> FillPath {
        FillPath::None
    }
}

impl_geometry_impl!(LineGeometryImpl);
