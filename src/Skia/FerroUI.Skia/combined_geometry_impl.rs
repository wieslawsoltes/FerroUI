use crate::geometry_impl::{
    impl_geometry_impl, register, try_get_geometry_impl, FillPath, GeometryImpl, GeometryImplBase,
};
use crate::skia_sharp_extensions::to_rect;
use ferroui_base::media::GeometryCombineMode;
use ferroui_base::platform::IGeometryImpl;
use ferroui_base::Rect;
use skia_safe::{Path, PathOp};
use std::rc::Rc;

/// A Skia implementation of a combined geometry.
pub struct CombinedGeometryImpl {
    base: GeometryImplBase,
    bounds: Rect,
    stroke_path: Option<Path>,
    fill: FillPath,
}

impl CombinedGeometryImpl {
    /// Creates a geometry from an already combined stroke and fill.
    pub fn new(stroke: Option<Path>, fill: FillPath) -> Rc<Self> {
        let mut bounds = stroke.as_ref().map(Path::compute_tight_bounds).unwrap_or_default();

        if let FillPath::Separate(fill) = &fill {
            let fill_bounds = fill.compute_tight_bounds();
            bounds = skia_safe::Rect::new(
                bounds.left.min(fill_bounds.left),
                bounds.top.min(fill_bounds.top),
                bounds.right.max(fill_bounds.right),
                bounds.bottom.max(fill_bounds.bottom),
            );
        }

        register(Self { base: GeometryImplBase::new(), bounds: to_rect(bounds), stroke_path: stroke, fill })
    }

    /// Combines two geometries; the result is empty when they cannot be
    /// combined.
    pub fn force_create(
        combine_mode: GeometryCombineMode,
        g1: &dyn IGeometryImpl,
        g2: &dyn IGeometryImpl,
    ) -> Rc<CombinedGeometryImpl> {
        if let (Some(i1), Some(i2)) = (try_get_geometry_impl(g1), try_get_geometry_impl(g2)) {
            if let Some(result) = Self::try_create(combine_mode, i1, i2) {
                return result;
            }
        }

        Self::new(None, FillPath::None)
    }

    /// Combines two geometries, or returns `None` when neither their strokes
    /// nor their fills can be combined.
    pub fn try_create(
        combine_mode: GeometryCombineMode,
        g1: &dyn GeometryImpl,
        g2: &dyn GeometryImpl,
    ) -> Option<Rc<CombinedGeometryImpl>> {
        let op = match combine_mode {
            GeometryCombineMode::Intersect => PathOp::Intersect,
            GeometryCombineMode::Xor => PathOp::XOR,
            GeometryCombineMode::Exclude => PathOp::Difference,
            _ => PathOp::Union,
        };

        let stroke = match (g1.stroke_path(), g2.stroke_path()) {
            (Some(s1), Some(s2)) => s1.op(&s2, op),
            _ => None,
        };

        let mut fill = FillPath::None;

        if let (Some(f1), Some(f2)) = (g1.fill_path(), g2.fill_path()) {
            // Reuse the stroke if the fill paths are the same.
            if g1.is_fill_same_as_stroke() && g2.is_fill_same_as_stroke() {
                if stroke.is_some() {
                    fill = FillPath::SameAsStroke;
                }
            } else if let Some(path) = f1.op(&f2, op) {
                fill = FillPath::Separate(path);
            }
        }

        if stroke.is_none() && matches!(fill, FillPath::None) {
            return None;
        }

        Some(Self::new(stroke, fill))
    }

    fn geometry_bounds(&self) -> Rect {
        self.bounds
    }
}

impl GeometryImpl for CombinedGeometryImpl {
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

impl_geometry_impl!(CombinedGeometryImpl);

#[cfg(test)]
mod tests {
    use super::*;
    use skia_safe::PathBuilder;

    #[test]
    fn combining_fill_with_empty_stroke_returns_fill_bounds() {
        let mut fill = PathBuilder::new();
        fill.line_to((100.0, 0.0));
        fill.line_to((100.0, 100.0));
        fill.line_to((0.0, 100.0));
        fill.close();

        let stroke = Path::new();

        let result = CombinedGeometryImpl::new(Some(stroke), FillPath::Separate(fill.detach()));

        assert_eq!(Rect::new(0.0, 0.0, 100.0, 100.0), result.bounds());
    }
}
