use super::ICompositionHitTester;
use crate::media::IntersectionResult;
use crate::platform::{IGeometryImpl, LtrbRect};
use crate::rendering::composition::CompositionVisual;
use crate::{Matrix, Point};

/// Hit testing with a point.
pub struct PointCompositionHitTester;

impl ICompositionHitTester for PointCompositionHitTester {
    type Input = Point;

    fn transform(input: &Point, matrix: Matrix) -> Point {
        input.transform(matrix)
    }

    fn hit_test(visual: &CompositionVisual, input: &Point) -> IntersectionResult {
        if visual.hit_test(*input) {
            IntersectionResult::FullyContains
        } else {
            IntersectionResult::Empty
        }
    }

    fn transformed_sub_tree_bounds_match(bounds: LtrbRect, input: &Point) -> bool {
        bounds.contains(input.x, input.y)
    }

    fn clipped_bounds_match(visual: &CompositionVisual, input: &Point) -> bool {
        let size = visual.size();
        input.x >= 0.0 && input.y >= 0.0 && input.x <= size.x && input.y <= size.y
    }

    fn clip_matches(clip: &dyn IGeometryImpl, input: &Point) -> bool {
        clip.fill_contains(*input)
    }
}
