use crate::geometry_impl::{
    impl_geometry_impl, register, try_get_geometry_impl, FillPath, GeometryImpl, GeometryImplBase, VelloPath,
};
use ferroui_base::media::FillRule;
use ferroui_base::platform::IGeometryImpl;
use ferroui_base::Rect;
use kurbo::BezPath;
use std::sync::Arc;

/// A kurbo implementation of a geometry group.
pub struct GeometryGroupImpl {
    base: GeometryImplBase,
    bounds: Rect,
    stroke_path: VelloPath,
    fill: FillPath,
}

impl GeometryGroupImpl {
    /// Creates a geometry from the paths of `children`. Children of another
    /// backend are skipped.
    pub fn new(fill_rule: FillRule, children: &[Arc<dyn IGeometryImpl>]) -> Arc<Self> {
        let children: Vec<&dyn GeometryImpl> =
            children.iter().filter_map(|child| try_get_geometry_impl(&**child)).collect();

        let mut stroke = BezPath::new();
        let mut requires_fill_pass = false;

        for geo in &children {
            if let Some(stroke_path) = geo.stroke_path() {
                stroke.extend(stroke_path.path().iter());
            }

            if !geo.is_fill_same_as_stroke() {
                requires_fill_pass = true;
            }
        }

        let stroke_path = VelloPath::new(stroke, fill_rule);

        let fill = if requires_fill_pass {
            let mut fill = BezPath::new();

            for geo in &children {
                if let Some(fill_path) = geo.fill_path() {
                    fill.extend(fill_path.path().iter());
                }
            }

            FillPath::Separate(VelloPath::new(fill, fill_rule))
        } else {
            FillPath::SameAsStroke
        };

        let bounds = stroke_path.tight_bounds();

        register(Self { base: GeometryImplBase::new(), bounds, stroke_path, fill })
    }

    fn geometry_bounds(&self) -> Rect {
        self.bounds
    }
}

impl GeometryImpl for GeometryGroupImpl {
    fn base(&self) -> &GeometryImplBase {
        &self.base
    }

    fn stroke_path(&self) -> Option<VelloPath> {
        Some(self.stroke_path.clone())
    }

    fn fill(&self) -> FillPath {
        self.fill.clone()
    }
}

impl_geometry_impl!(GeometryGroupImpl);
