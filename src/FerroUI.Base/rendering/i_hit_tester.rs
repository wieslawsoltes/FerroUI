use crate::media::{Geometry, GeometryHitTestResult};
use crate::{Point, Ref, Visual};

/// An interface to allow hit testing of a visual tree.
///
/// Hit testing through this interface is low-level and does not respect
/// input hit test visibility. Use the input hit testing methods of
/// `InputElement` to perform input hit testing, or provide your own filter.
pub trait IHitTester {
    /// Hit tests a location to find the visuals at the specified point,
    /// topmost first.
    ///
    /// `p` is the point in coordinates relative to `root`, the root of the
    /// subtree to search. If `filter` returns false for a visual then the
    /// visual and all its descendants are excluded from the results.
    fn hit_test(&self, p: Point, root: &Visual, filter: Option<&dyn Fn(&Visual) -> bool>) -> Vec<Ref<Visual>>;

    /// Hit tests a geometry to find the visuals intersecting a region,
    /// topmost first.
    ///
    /// `geometry` is in coordinates relative to `root`, the root of the
    /// subtree to search. If `filter` returns false for a visual then the
    /// visual and all its descendants are excluded from the results.
    fn hit_test_geometry(
        &self,
        geometry: &Geometry,
        root: &Visual,
        filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<GeometryHitTestResult>;

    /// Hit tests a location to find the first (topmost) visual at the
    /// specified point.
    ///
    /// See [`hit_test`](Self::hit_test) for the parameters.
    fn hit_test_first(&self, p: Point, root: &Visual, filter: Option<&dyn Fn(&Visual) -> bool>)
        -> Option<Ref<Visual>>;

    /// Hit tests a geometry to find the first (topmost) visual intersecting
    /// a region.
    ///
    /// See [`hit_test_geometry`](Self::hit_test_geometry) for the
    /// parameters.
    fn hit_test_first_geometry(
        &self,
        geometry: &Geometry,
        root: &Visual,
        filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<GeometryHitTestResult>;
}
