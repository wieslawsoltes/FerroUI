use crate::media::IntersectionResult;
use crate::platform::{IGeometryImpl, LtrbRect};
use crate::rendering::composition::drawing::CompositionRenderData;
use crate::rendering::composition::CompositionVisual;
use crate::Matrix;

/// A kind of hit test of the composition visual tree: what is tested (a
/// point, a geometry) and how it relates to visuals.
pub trait ICompositionHitTester {
    type Input: Clone;

    fn transform(input: &Self::Input, matrix: Matrix) -> Self::Input;

    fn hit_test(visual: &CompositionVisual, input: &Self::Input) -> IntersectionResult;

    /// How `input`, in the coordinates of a drawing, hits it. What a visual without a
    /// composition visual drew is tested with it, where the composition visual of a visual
    /// above shows it (docs/porting/desktop-performance.md, section 9.42). A tester that does
    /// not say hits no such drawing.
    fn hit_test_drawing(_data: &CompositionRenderData, _input: &Self::Input) -> IntersectionResult {
        IntersectionResult::Empty
    }

    /// Whether `input`, in the coordinates of a composition visual, can hit a drawing whose
    /// bounds there `bounds` gives. A composition visual is passed when the input does not meet
    /// the bounds of what it shows, and what a visual without one drew is passed the same way
    /// (section 9.42). A tester whose test of a drawing says as much does not ask for the
    /// bounds.
    fn drawing_bounds_match(_bounds: &dyn Fn() -> Option<LtrbRect>, _input: &Self::Input) -> bool {
        true
    }

    fn transformed_sub_tree_bounds_match(bounds: LtrbRect, input: &Self::Input) -> bool;

    fn clipped_bounds_match(visual: &CompositionVisual, input: &Self::Input) -> bool;

    fn clip_matches(clip: &dyn IGeometryImpl, input: &Self::Input) -> bool;
}

/// `result > IntersectionResult.Empty`.
pub fn is_hit(result: IntersectionResult) -> bool {
    (result as i32) > (IntersectionResult::Empty as i32)
}
