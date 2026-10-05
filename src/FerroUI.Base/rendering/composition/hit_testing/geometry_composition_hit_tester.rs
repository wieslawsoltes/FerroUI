use super::{is_hit, ICompositionHitTester};
use crate::media::{Geometry, IntersectionResult, MatrixTransform};
use crate::platform::{IGeometryImpl, LtrbRect};
use crate::rendering::composition::CompositionVisual;
use crate::{Matrix, Rect, Ref, Size};

/// Hit testing with a geometry.
pub struct GeometryCompositionHitTester;

impl ICompositionHitTester for GeometryCompositionHitTester {
    type Input = Ref<Geometry>;

    fn transform(input: &Ref<Geometry>, matrix: Matrix) -> Ref<Geometry> {
        let result = input.clone_geometry();
        let current = input.transform().map_or(Matrix::IDENTITY, |transform| transform.value());
        result.set_transform(MatrixTransform::with_matrix(current * matrix));
        result
    }

    fn hit_test(visual: &CompositionVisual, input: &Ref<Geometry>) -> IntersectionResult {
        visual.hit_test_geometry(input)
    }

    fn transformed_sub_tree_bounds_match(bounds: LtrbRect, input: &Ref<Geometry>) -> bool {
        let geometry_render_bounds = input.bounds();
        bounds.intersects(LtrbRect::from_rect(geometry_render_bounds))
    }

    fn clipped_bounds_match(visual: &CompositionVisual, input: &Ref<Geometry>) -> bool {
        let bounds = input.bounds();
        let size = visual.size();
        bounds.width > 0.0 && bounds.height > 0.0 && bounds.intersects(Rect::from_size(Size::new(size.x, size.y)))
    }

    fn clip_matches(clip: &dyn IGeometryImpl, input: &Ref<Geometry>) -> bool {
        input.platform_impl().is_some_and(|geometry_impl| is_hit(clip.get_fill_intersection_result(&*geometry_impl)))
    }
}
