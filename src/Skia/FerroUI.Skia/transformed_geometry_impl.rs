use crate::geometry_impl::{impl_geometry_impl, register, FillPath, GeometryImpl, GeometryImplBase};
use crate::skia_sharp_extensions::{to_rect, to_sk_matrix};
use ferroui_base::platform::{IGeometryImpl, ITransformedGeometryImpl};
use ferroui_base::{Matrix, Rect};
use skia_safe::Path;
use std::rc::Rc;

/// A Skia implementation of a transformed geometry.
pub struct TransformedGeometryImpl {
    base: GeometryImplBase,
    bounds: Rect,
    stroke_path: Option<Path>,
    fill: FillPath,
    source_geometry: Rc<dyn GeometryImpl>,
    transform: Matrix,
}

impl TransformedGeometryImpl {
    /// Creates a geometry that is `source` with `transform` applied.
    pub fn new(source: Rc<dyn GeometryImpl>, transform: Matrix) -> Rc<Self> {
        let matrix = to_sk_matrix(transform);

        let stroke_path = source.stroke_path().map(|path| path.make_transform(&matrix));
        let bounds = stroke_path.as_ref().map(|path| to_rect(path.compute_tight_bounds())).unwrap_or_default();

        let fill = match source.fill() {
            FillPath::SameAsStroke if stroke_path.is_some() => FillPath::SameAsStroke,
            FillPath::Separate(fill_path) => FillPath::Separate(fill_path.make_transform(&matrix)),
            _ => FillPath::None,
        };

        register(Self {
            base: GeometryImplBase::new(),
            bounds,
            stroke_path,
            fill,
            source_geometry: source,
            transform,
        })
    }

    fn geometry_bounds(&self) -> Rect {
        self.bounds
    }
}

impl GeometryImpl for TransformedGeometryImpl {
    fn base(&self) -> &GeometryImplBase {
        &self.base
    }

    fn stroke_path(&self) -> Option<Path> {
        self.stroke_path.clone()
    }

    fn fill(&self) -> FillPath {
        self.fill.clone()
    }
}

impl ITransformedGeometryImpl for TransformedGeometryImpl {
    fn source_geometry(&self) -> Rc<dyn IGeometryImpl> {
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
