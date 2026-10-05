use crate::media::{Geometry, IntersectionResult};
use crate::{Point, Ref};

/// An interface to allow non-templated controls to customize their hit
/// testing when using a renderer with a simple hit-testing algorithm, where
/// just the bounds or geometries are hit-tested.
pub trait ICustomHitTest {
    /// Whether the control is hit at the specified point, in coordinates
    /// relative to the control.
    fn hit_test(&self, point: Point) -> bool;

    /// The intersection of the control with a geometry.
    fn hit_test_geometry(&self, _geometry: &Ref<Geometry>) -> IntersectionResult {
        IntersectionResult::Empty
    }
}
