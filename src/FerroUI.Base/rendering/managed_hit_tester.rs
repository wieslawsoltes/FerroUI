use super::IHitTester;
use crate::media::{Geometry, GeometryHitTestResult, IntersectionResult, MatrixTransform, RectangleGeometry};
use crate::visual_tree::local_transform;
use crate::{Matrix, Point, Rect, Ref, Visual};

/// A hit tester that walks the visual tree directly.
///
/// A visual is hit when the point lies within its bounds. Subtrees are
/// skipped when the visual is invisible, rejected by the filter, or clips
/// its content to its bounds and the point lies outside of them, or has a
/// clip geometry that does not contain the point. Children
/// are tested before their parent and in front-to-back order: by descending
/// Z index, then by descending position in the visual children.
///
/// Hit testing with a geometry follows the same rules with the geometry in
/// place of the point: a visual is hit when the rectangle of its bounds
/// and the geometry intersect, and the result tells how. It needs the
/// geometry implementations of the render interface.
///
/// This is the fallback for presentation sources without a renderer-backed
/// hit tester. It only knows about bounds: it does not look at what a
/// visual actually draws, so a visual is hit anywhere within its bounds
/// even where it draws nothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct ManagedHitTester;

impl ManagedHitTester {
    /// Creates the hit tester.
    pub const fn new() -> Self {
        Self
    }

    /// Visits the visuals at `p` (in the coordinates of `visual`), topmost
    /// first. Returns true if `visit` asked to stop.
    fn hit_test_core(
        visual: &Visual,
        p: Point,
        filter: Option<&dyn Fn(&Visual) -> bool>,
        visit: &mut dyn FnMut(&Visual) -> bool,
    ) -> bool {
        if !visual.is_visible() {
            return false;
        }

        if let Some(filter) = filter {
            if !filter(visual) {
                return false;
            }
        }

        let contains_point = Rect::from_size(visual.bounds().size()).contains(p);

        if visual.clip_to_bounds() && !contains_point {
            return false;
        }

        if let Some(clip) = visual.clip() {
            if !clip.fill_contains(p) {
                return false;
            }
        }

        if let Some(children) = visual.visual_children_snapshot() {
            if visual.has_non_uniform_z_index_children() {
                for child in Visual::sort_by_z_index(&children).iter() {
                    if Self::hit_test_child(child, p, filter, visit) {
                        return true;
                    }
                }
            } else {
                for child in children.iter().rev() {
                    if Self::hit_test_child(child, p, filter, visit) {
                        return true;
                    }
                }
            }
        }

        // A visual with a custom hit test decides for itself.
        let is_hit = visual.custom_hit_test(p).unwrap_or(contains_point);

        is_hit && visit(visual)
    }

    fn hit_test_child(
        child: &Visual,
        parent_point: Point,
        filter: Option<&dyn Fn(&Visual) -> bool>,
        visit: &mut dyn FnMut(&Visual) -> bool,
    ) -> bool {
        match local_transform(child).try_invert() {
            Some(inverted) => Self::hit_test_core(child, parent_point.transform(inverted), filter, visit),
            None => false,
        }
    }

    /// Returns `input` in the coordinate space that `matrix` maps the
    /// current one to.
    fn transform_geometry(input: &Geometry, matrix: Matrix) -> Ref<Geometry> {
        let result = input.clone_geometry();
        let current = input.transform().map_or(Matrix::IDENTITY, |transform| transform.value());
        result.set_transform(MatrixTransform::with_matrix(current * matrix));
        result
    }

    fn is_hit(result: IntersectionResult) -> bool {
        !matches!(result, IntersectionResult::NotCalculated | IntersectionResult::Empty)
    }

    /// Visits the visuals intersecting `geometry` (in the coordinates of
    /// `visual`), topmost first. Returns true if `visit` asked to stop.
    fn hit_test_geometry_core(
        visual: &Visual,
        geometry: &Geometry,
        filter: Option<&dyn Fn(&Visual) -> bool>,
        visit: &mut dyn FnMut(&Visual, IntersectionResult) -> bool,
    ) -> bool {
        if !visual.is_visible() {
            return false;
        }

        if let Some(filter) = filter {
            if !filter(visual) {
                return false;
            }
        }

        let bounds = Rect::from_size(visual.bounds().size());

        if visual.clip_to_bounds() {
            let geometry_bounds = geometry.bounds();
            if !(geometry_bounds.width > 0.0 && geometry_bounds.height > 0.0 && geometry_bounds.intersects(bounds)) {
                return false;
            }
        }

        if let Some(clip) = visual.clip() {
            if !clip.get_fill_intersection_result(geometry).is_some_and(Self::is_hit) {
                return false;
            }
        }

        if let Some(children) = visual.visual_children_snapshot() {
            if visual.has_non_uniform_z_index_children() {
                for child in Visual::sort_by_z_index(&children).iter() {
                    if Self::hit_test_geometry_child(child, geometry, filter, visit) {
                        return true;
                    }
                }
            } else {
                for child in children.iter().rev() {
                    if Self::hit_test_geometry_child(child, geometry, filter, visit) {
                        return true;
                    }
                }
            }
        }

        // The relation between the hit geometry and the rectangle the
        // visual occupies.
        if let Some(result) = visual.custom_hit_test_geometry(&geometry.to_ref()) {
            return Self::is_hit(result) && visit(visual, result);
        }

        let target = RectangleGeometry::with_rect(bounds);
        match geometry.get_fill_intersection_result(&target) {
            Some(result) if Self::is_hit(result) => visit(visual, result),
            _ => false,
        }
    }

    fn hit_test_geometry_child(
        child: &Visual,
        parent_geometry: &Geometry,
        filter: Option<&dyn Fn(&Visual) -> bool>,
        visit: &mut dyn FnMut(&Visual, IntersectionResult) -> bool,
    ) -> bool {
        match local_transform(child).try_invert() {
            Some(inverted) => {
                let geometry = Self::transform_geometry(parent_geometry, inverted);
                Self::hit_test_geometry_core(child, &geometry, filter, visit)
            }
            None => false,
        }
    }
}

impl IHitTester for ManagedHitTester {
    fn hit_test(&self, p: Point, root: &Visual, filter: Option<&dyn Fn(&Visual) -> bool>) -> Vec<Ref<Visual>> {
        let mut result = Vec::new();
        Self::hit_test_core(root, p, filter, &mut |visual| {
            result.push(visual.to_ref());
            false
        });
        result
    }

    fn hit_test_first(
        &self,
        p: Point,
        root: &Visual,
        filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<Ref<Visual>> {
        let mut result = None;
        Self::hit_test_core(root, p, filter, &mut |visual| {
            result = Some(visual.to_ref());
            true
        });
        result
    }

    fn hit_test_geometry(
        &self,
        geometry: &Geometry,
        root: &Visual,
        filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<GeometryHitTestResult> {
        let mut result = Vec::new();
        Self::hit_test_geometry_core(root, geometry, filter, &mut |visual, intersection| {
            result.push(GeometryHitTestResult::new(visual.to_ref(), intersection));
            false
        });
        result
    }

    fn hit_test_first_geometry(
        &self,
        geometry: &Geometry,
        root: &Visual,
        filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<GeometryHitTestResult> {
        let mut result = None;
        Self::hit_test_geometry_core(root, geometry, filter, &mut |visual, intersection| {
            result = Some(GeometryHitTestResult::new(visual.to_ref(), intersection));
            true
        });
        result
    }
}
