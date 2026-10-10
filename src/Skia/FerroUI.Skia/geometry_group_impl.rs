use crate::geometry_impl::{
    impl_geometry_impl, register, try_get_geometry_impl, FillPath, GeometryImpl, GeometryImplBase, Shared,
};
use crate::skia_sharp_extensions::to_rect;
use ferroui_base::media::FillRule;
use ferroui_base::platform::IGeometryImpl;
use ferroui_base::Rect;
use skia_safe::{Path, PathBuilder, PathFillType};
use std::sync::Arc;

/// A Skia implementation of a geometry group.
pub struct GeometryGroupImpl {
    base: GeometryImplBase,
    bounds: Rect,
    stroke_path: Shared<Path>,
    fill: Shared<FillPath>,
}

impl GeometryGroupImpl {
    /// Creates a geometry from the paths of `children`. Children of another
    /// backend are skipped.
    pub fn new(fill_rule: FillRule, children: &[Arc<dyn IGeometryImpl>]) -> Arc<Self> {
        let fill_type = if fill_rule == FillRule::NonZero { PathFillType::Winding } else { PathFillType::EvenOdd };
        let children: Vec<&dyn GeometryImpl> =
            children.iter().filter_map(|child| try_get_geometry_impl(&**child)).collect();

        let mut stroke = PathBuilder::new_with_fill_type(fill_type);
        let mut requires_fill_pass = false;

        for geo in &children {
            if let Some(stroke_path) = geo.stroke_path() {
                stroke.add_path(&stroke_path, None);
            }

            if !geo.is_fill_same_as_stroke() {
                requires_fill_pass = true;
            }
        }

        let stroke_path = stroke.detach();

        let fill = if requires_fill_pass {
            let mut fill = PathBuilder::new_with_fill_type(fill_type);

            for geo in &children {
                if let Some(fill_path) = geo.fill_path() {
                    fill.add_path(&fill_path, None);
                }
            }

            FillPath::Separate(fill.detach())
        } else {
            FillPath::SameAsStroke
        };

        let bounds = to_rect(crate::skia_sharp_extensions::tight_bounds(&stroke_path));

        register(Self { base: GeometryImplBase::new(), bounds, stroke_path: Shared::new(stroke_path), fill: Shared::new(fill) })
    }

    fn geometry_bounds(&self) -> Rect {
        self.bounds
    }
}

impl GeometryImpl for GeometryGroupImpl {
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

impl_geometry_impl!(GeometryGroupImpl);
