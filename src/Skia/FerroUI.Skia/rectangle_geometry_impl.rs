use crate::geometry_impl::{impl_geometry_impl, register, FillPath, GeometryImpl, GeometryImplBase};
use crate::skia_sharp_extensions::to_sk_rect;
use ferroui_base::Rect;
use skia_safe::{Path, PathBuilder};
use std::rc::Rc;

/// A Skia implementation of a rectangle geometry.
pub struct RectangleGeometryImpl {
    base: GeometryImplBase,
    bounds: Rect,
    stroke_path: Path,
}

impl RectangleGeometryImpl {
    /// Creates the geometry of `rect`.
    pub fn new(rect: Rect) -> Rc<Self> {
        let mut path = PathBuilder::new();
        path.add_rect(to_sk_rect(rect), None, None);

        register(Self { base: GeometryImplBase::new(), bounds: rect, stroke_path: path.detach() })
    }

    fn geometry_bounds(&self) -> Rect {
        self.bounds
    }
}

impl GeometryImpl for RectangleGeometryImpl {
    fn base(&self) -> &GeometryImplBase {
        &self.base
    }

    fn stroke_path(&self) -> Option<Path> {
        Some(self.stroke_path.clone())
    }

    fn fill(&self) -> FillPath {
        FillPath::SameAsStroke
    }
}

impl_geometry_impl!(RectangleGeometryImpl);
