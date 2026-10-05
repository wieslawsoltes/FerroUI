use crate::media::IntersectionResult;
use crate::{Ref, Visual};

/// Returns the results of a hit test that uses a geometry as a hit test parameter.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GeometryHitTestResult {
    /// The [`Visual`] that is returned from a hit test result.
    pub visual_hit: Ref<Visual>,
    /// The [`IntersectionResult`] value of the hit test.
    pub intersection_result: IntersectionResult,
}

impl GeometryHitTestResult {
    pub fn new(visual_hit: Ref<Visual>, intersection_result: IntersectionResult) -> Self {
        Self { visual_hit, intersection_result }
    }
}
