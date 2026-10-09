use crate::geometry_impl::{impl_geometry_impl, register, FillPath, GeometryImpl, GeometryImplBase, VelloPath};
use crate::vello_extensions::rect_path;
use ferroui_base::media::FillRule;
use ferroui_base::Rect;
use std::sync::Arc;

/// A kurbo implementation of a rectangle geometry.
pub struct RectangleGeometryImpl {
    base: GeometryImplBase,
    bounds: Rect,
    stroke_path: VelloPath,
}

impl RectangleGeometryImpl {
    /// Creates the geometry of `rect`.
    pub fn new(rect: Rect) -> Arc<Self> {
        let path = VelloPath::new(rect_path(rect), FillRule::NonZero);

        register(Self { base: GeometryImplBase::new(), bounds: rect, stroke_path: path })
    }

    fn geometry_bounds(&self) -> Rect {
        self.bounds
    }
}

impl GeometryImpl for RectangleGeometryImpl {
    fn base(&self) -> &GeometryImplBase {
        &self.base
    }

    fn stroke_path(&self) -> Option<VelloPath> {
        Some(self.stroke_path.clone())
    }

    fn fill(&self) -> FillPath {
        FillPath::SameAsStroke
    }
}

impl_geometry_impl!(RectangleGeometryImpl);
