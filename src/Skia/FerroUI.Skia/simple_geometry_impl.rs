use crate::geometry_impl::{
    cache_path_bounds, impl_geometry_impl, register, FillPath, GeometryImpl, GeometryImplBase, Shared,
};
use crate::skia_sharp_extensions::{tight_bounds, to_rect};
use ferroui_base::Rect;
use skia_safe::Path;
use std::sync::Arc;

/// A Skia geometry defined by stroke and fill paths.
pub struct SimpleGeometryImpl {
    base: GeometryImplBase,
    bounds: Rect,
    stroke_path: Shared<Path>,
    fill: Shared<FillPath>,
}

impl SimpleGeometryImpl {
    /// Creates a geometry from a stroke path and a fill. `bounds` defaults to
    /// the tight bounds of the stroke path.
    pub fn new(stroke_path: Path, fill: FillPath, bounds: Option<Rect>) -> Arc<Self> {
        let bounds = bounds.unwrap_or_else(|| to_rect(tight_bounds(&stroke_path)));

        let fill = match fill {
            FillPath::Separate(path) => FillPath::Separate(cache_path_bounds(path)),
            other => other,
        };

        register(Self {
            base: GeometryImplBase::new(),
            bounds,
            stroke_path: Shared::new(cache_path_bounds(stroke_path)),
            fill: Shared::new(fill),
        })
    }

    fn geometry_bounds(&self) -> Rect {
        self.bounds
    }
}

impl GeometryImpl for SimpleGeometryImpl {
    fn base(&self) -> &GeometryImplBase {
        &self.base
    }

    fn stroke_path(&self) -> Option<Path> {
        Some(self.stroke_path.get())
    }

    fn fill(&self) -> FillPath {
        self.fill.get()
    }
}

impl_geometry_impl!(SimpleGeometryImpl);
