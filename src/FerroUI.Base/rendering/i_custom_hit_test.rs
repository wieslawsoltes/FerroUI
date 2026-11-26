use crate::media::{Geometry, IntersectionResult};
use crate::{Point, Ref};

/// Allows hit testing to use a custom area instead of the area of the rendered content.
pub trait ICustomHitTest {
    /// Whether a point is within the hit-test area. The point is in local coordinates: (0, 0)
    /// is the top-left corner of the visual element, not the top-left corner of the window.
    fn hit_test(&self, point: Point) -> bool;

    /// How a geometry, mapped to local coordinates, intersects the hit-test area. Children are
    /// not tested: test them by hand if needed.
    fn hit_test_geometry(&self, _geometry: &Ref<Geometry>) -> IntersectionResult {
        IntersectionResult::Empty
    }
}
