use crate::media::IntersectionResult;
use crate::platform::{IGeometryImpl, LtrbRect};
use crate::rendering::composition::CompositionVisual;
use crate::Matrix;

/// A kind of hit test of the composition visual tree: what is tested (a
/// point, a geometry) and how it relates to visuals.
pub trait ICompositionHitTester {
    type Input: Clone;

    fn transform(input: &Self::Input, matrix: Matrix) -> Self::Input;

    fn hit_test(visual: &CompositionVisual, input: &Self::Input) -> IntersectionResult;

    fn transformed_sub_tree_bounds_match(bounds: LtrbRect, input: &Self::Input) -> bool;

    fn clipped_bounds_match(visual: &CompositionVisual, input: &Self::Input) -> bool;

    fn clip_matches(clip: &dyn IGeometryImpl, input: &Self::Input) -> bool;
}

/// `result > IntersectionResult.Empty`.
pub fn is_hit(result: IntersectionResult) -> bool {
    (result as i32) > (IntersectionResult::Empty as i32)
}
