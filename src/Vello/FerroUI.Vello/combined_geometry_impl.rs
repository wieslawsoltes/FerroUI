use crate::geometry_impl::{
    impl_geometry_impl, register, try_get_geometry_impl, FillPath, GeometryImpl, GeometryImplBase, VelloPath,
};
use crate::vello_extensions::closed_path;
use ferroui_base::media::{FillRule, GeometryCombineMode};
use ferroui_base::platform::IGeometryImpl;
use ferroui_base::Rect;
use kurbo::{BezPath, PathEl, Shape};
use linesweeper::BinaryOp;
use std::sync::Arc;

/// A kurbo implementation of a combined geometry.
///
/// kurbo has no boolean operations of paths; they are those of the
/// `linesweeper` crate, a sweep-line algorithm over the Bézier paths of
/// kurbo (design document, section 1.4).
pub struct CombinedGeometryImpl {
    base: GeometryImplBase,
    bounds: Rect,
    stroke_path: Option<VelloPath>,
    fill: FillPath,
}

impl CombinedGeometryImpl {
    /// Creates a geometry from an already combined stroke and fill.
    pub fn new(stroke: Option<VelloPath>, fill: FillPath) -> Arc<Self> {
        let mut bounds = stroke.as_ref().map(VelloPath::tight_bounds).unwrap_or_default();

        if let FillPath::Separate(fill) = &fill {
            let fill_bounds = fill.tight_bounds();
            let (left, top) = (bounds.x.min(fill_bounds.x), bounds.y.min(fill_bounds.y));
            let (right, bottom) =
                (bounds.right().max(fill_bounds.right()), bounds.bottom().max(fill_bounds.bottom()));
            bounds = Rect::new(left, top, right - left, bottom - top);
        }

        register(Self { base: GeometryImplBase::new(), bounds, stroke_path: stroke, fill })
    }

    /// Combines two geometries; the result is empty when they cannot be
    /// combined.
    pub fn force_create(
        combine_mode: GeometryCombineMode,
        g1: &dyn IGeometryImpl,
        g2: &dyn IGeometryImpl,
    ) -> Arc<CombinedGeometryImpl> {
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
    ) -> Option<Arc<CombinedGeometryImpl>> {
        let stroke = match (g1.stroke_path(), g2.stroke_path()) {
            (Some(s1), Some(s2)) => combine_paths(combine_mode, &s1, &s2),
            _ => None,
        };

        let mut fill = FillPath::None;

        if let (Some(f1), Some(f2)) = (g1.fill_path(), g2.fill_path()) {
            // Reuse the stroke if the fill paths are the same.
            if g1.is_fill_same_as_stroke() && g2.is_fill_same_as_stroke() {
                if stroke.is_some() {
                    fill = FillPath::SameAsStroke;
                }
            } else if let Some(path) = combine_paths(combine_mode, &f1, &f2) {
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

/// The area a path encloses, whichever way its figures run.
pub(crate) fn path_area(path: &BezPath) -> f64 {
    path.area().abs()
}

/// The area of two paths combined, as a path of simple, closed figures that
/// either fill rule fills alike. Open figures are closed first, as filling
/// closes them. `None` when the operation fails.
pub(crate) fn combine_paths(combine_mode: GeometryCombineMode, a: &VelloPath, b: &VelloPath) -> Option<VelloPath> {
    let op = match combine_mode {
        GeometryCombineMode::Intersect => BinaryOp::Intersection,
        GeometryCombineMode::Xor => BinaryOp::Xor,
        GeometryCombineMode::Exclude => BinaryOp::Difference,
        _ => BinaryOp::Union,
    };

    let (path_a, path_b) = (closed_path(a.path()), closed_path(b.path()));

    // The operation takes one fill rule for both paths: paths of different
    // rules are first reduced to figures that both rules fill alike.
    let (path_a, path_b, fill_rule) = if a.fill_rule() == b.fill_rule() {
        (path_a, path_b, a.fill_rule())
    } else {
        (
            binary_op(&path_a, &BezPath::new(), a.fill_rule(), BinaryOp::Union)?,
            binary_op(&path_b, &BezPath::new(), b.fill_rule(), BinaryOp::Union)?,
            FillRule::NonZero,
        )
    };

    Some(VelloPath::new(binary_op(&path_a, &path_b, fill_rule, op)?, FillRule::NonZero))
}

fn binary_op(a: &BezPath, b: &BezPath, fill_rule: FillRule, op: BinaryOp) -> Option<BezPath> {
    let fill_rule =
        if fill_rule == FillRule::EvenOdd { linesweeper::FillRule::EvenOdd } else { linesweeper::FillRule::NonZero };

    // The sweep is in an early state by its own account and may give up on
    // an input by panicking; a combination that fails is the empty geometry
    // of the contract, not the end of the application. The inputs it is known
    // to refuse that way, coordinates that are not finite, are refused here
    // first, so that a build that aborts on panic does not reach the sweep
    // with them (#567).
    if !is_finite(a) || !is_finite(b) {
        return None;
    }
    let contours = std::panic::catch_unwind(|| linesweeper::binary_op(a, b, fill_rule, op)).ok()?.ok()?;

    let mut path = BezPath::new();
    for contour in contours.contours() {
        path.extend(contour.path.iter());
    }

    Some(path)
}

/// Whether every point of the path has finite coordinates.
fn is_finite(path: &BezPath) -> bool {
    path.elements().iter().all(|element| match *element {
        PathEl::MoveTo(p) | PathEl::LineTo(p) => p.is_finite(),
        PathEl::QuadTo(p1, p2) => p1.is_finite() && p2.is_finite(),
        PathEl::CurveTo(p1, p2, p3) => p1.is_finite() && p2.is_finite() && p3.is_finite(),
        PathEl::ClosePath => true,
    })
}

impl GeometryImpl for CombinedGeometryImpl {
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

impl_geometry_impl!(CombinedGeometryImpl);

#[cfg(test)]
mod tests {
    // Not from upstream: the refusal of non-finite inputs is of this port.
    use super::*;
    use kurbo::Rect as KurboRect;

    fn rect_path(x: f64) -> VelloPath {
        VelloPath::new(KurboRect::new(x, 0.0, x + 10.0, 10.0).to_path(0.1), FillRule::NonZero)
    }

    #[test]
    fn a_path_with_coordinates_that_are_not_finite_combines_to_nothing() {
        let mut nan = BezPath::new();
        nan.move_to((0.0, 0.0));
        nan.line_to((f64::NAN, 5.0));
        nan.line_to((5.0, f64::INFINITY));
        nan.close_path();
        let nan = VelloPath::new(nan, FillRule::NonZero);

        assert!(combine_paths(GeometryCombineMode::Union, &nan, &rect_path(0.0)).is_none());
        assert!(combine_paths(GeometryCombineMode::Intersect, &rect_path(0.0), &nan).is_none());
        assert!(combine_paths(GeometryCombineMode::Union, &rect_path(0.0), &rect_path(5.0)).is_some());
    }
}
