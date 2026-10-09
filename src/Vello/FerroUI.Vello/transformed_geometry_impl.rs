use crate::geometry_impl::{impl_geometry_impl, register, FillPath, GeometryImpl, GeometryImplBase, VelloPath};
use crate::vello_extensions::to_affine;
use ferroui_base::platform::{IGeometryImpl, ITransformedGeometryImpl};
use ferroui_base::{Matrix, Rect};
use std::sync::Arc;

/// A kurbo implementation of a transformed geometry.
pub struct TransformedGeometryImpl {
    base: GeometryImplBase,
    bounds: Rect,
    stroke_path: Option<VelloPath>,
    fill: FillPath,
    source_geometry: Arc<dyn GeometryImpl>,
    transform: Matrix,
}

impl TransformedGeometryImpl {
    /// Creates a geometry that is `source` with `transform` applied.
    pub fn new(source: Arc<dyn GeometryImpl>, transform: Matrix) -> Arc<Self> {
        let affine = to_affine(transform);

        let stroke_path = source.stroke_path().map(|path| path.transformed(affine));
        let bounds = stroke_path.as_ref().map(VelloPath::tight_bounds).unwrap_or_default();

        let fill = match source.fill() {
            FillPath::SameAsStroke if stroke_path.is_some() => FillPath::SameAsStroke,
            FillPath::Separate(fill_path) => FillPath::Separate(fill_path.transformed(affine)),
            _ => FillPath::None,
        };

        register(Self { base: GeometryImplBase::new(), bounds, stroke_path, fill, source_geometry: source, transform })
    }

    fn geometry_bounds(&self) -> Rect {
        self.bounds
    }
}

impl GeometryImpl for TransformedGeometryImpl {
    fn base(&self) -> &GeometryImplBase {
        &self.base
    }

    fn stroke_path(&self) -> Option<VelloPath> {
        self.stroke_path.clone()
    }

    fn fill(&self) -> FillPath {
        self.fill.clone()
    }
}

impl ITransformedGeometryImpl for TransformedGeometryImpl {
    fn source_geometry(&self) -> Arc<dyn IGeometryImpl> {
        self.source_geometry.clone()
    }

    fn transform(&self) -> Matrix {
        self.transform
    }
}

impl_geometry_impl!(TransformedGeometryImpl, {
    fn as_transformed_geometry(&self) -> Option<&dyn ITransformedGeometryImpl> {
        Some(self)
    }
});
