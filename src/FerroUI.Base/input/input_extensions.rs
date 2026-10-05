//! Input hit testing: extension methods of [`InputElement`].

use super::InputElement;
use crate::media::{Geometry, GeometryHitTestResult};
use crate::{Point, Ref, Visual};

fn is_hit_test_visible(visual: &Visual) -> bool {
    visual.is_visible()
        && visual.is_attached_to_visual_tree()
        && visual.downcast_ref::<InputElement>().is_some_and(InputElement::is_hit_test_visible)
}

fn is_hit_test_visible_enabled_only(visual: &Visual) -> bool {
    is_hit_test_visible(visual)
        && visual.downcast_ref::<InputElement>().is_some_and(InputElement::is_effectively_enabled)
}

fn hit_test_filter(enabled_elements_only: bool) -> fn(&Visual) -> bool {
    if enabled_elements_only {
        is_hit_test_visible_enabled_only
    } else {
        is_hit_test_visible
    }
}

impl InputElement {
    /// Returns the active input elements at a point on the element, in the
    /// element's coordinates, topmost first. Only enabled elements are
    /// considered.
    pub fn get_input_elements_at(&self, p: Point) -> Vec<Ref<InputElement>> {
        self.get_input_elements_at_with(p, true)
    }

    /// Returns the active input elements at a point on the element, in the
    /// element's coordinates, topmost first.
    ///
    /// `enabled_elements_only` tells whether only enabled elements should
    /// be considered.
    pub fn get_input_elements_at_with(&self, p: Point, enabled_elements_only: bool) -> Vec<Ref<InputElement>> {
        self.get_visuals_at_filtered(p, &hit_test_filter(enabled_elements_only))
            .into_iter()
            .filter_map(|visual| visual.downcast::<InputElement>().ok())
            .collect()
    }

    /// Returns the active input elements intersecting a geometry on the
    /// element, in the element's coordinates, topmost first, with the
    /// details of the intersections.
    ///
    /// `enabled_elements_only` tells whether only enabled elements should
    /// be considered.
    pub fn get_input_elements_at_geometry(
        &self,
        geometry: &Geometry,
        enabled_elements_only: bool,
    ) -> Vec<GeometryHitTestResult> {
        self.get_visuals_at_geometry_filtered(geometry, &hit_test_filter(enabled_elements_only))
            .into_iter()
            .filter(|x| x.visual_hit.is::<InputElement>())
            .collect()
    }

    /// Returns the topmost active input element at a point on the element,
    /// in the element's coordinates. Only enabled elements are considered.
    pub fn input_hit_test(&self, p: Point) -> Option<Ref<InputElement>> {
        self.input_hit_test_with(p, true)
    }

    /// Returns the topmost active input element at a point on the element,
    /// in the element's coordinates.
    ///
    /// `enabled_elements_only` tells whether only enabled elements should
    /// be considered.
    pub fn input_hit_test_with(&self, p: Point, enabled_elements_only: bool) -> Option<Ref<InputElement>> {
        self.get_visual_at_filtered(p, &hit_test_filter(enabled_elements_only))
            .and_then(|visual| visual.downcast::<InputElement>().ok())
    }

    /// Returns the topmost active input element intersecting a geometry on
    /// the element, in the element's coordinates, with the details of the
    /// intersection.
    ///
    /// `enabled_elements_only` tells whether only enabled elements should
    /// be considered.
    pub fn input_hit_test_geometry(
        &self,
        geometry: &Geometry,
        enabled_elements_only: bool,
    ) -> Option<GeometryHitTestResult> {
        self.get_visual_at_geometry_filtered(geometry, &hit_test_filter(enabled_elements_only))
            .filter(|hit_test| hit_test.visual_hit.is::<InputElement>())
    }

    /// Returns the topmost active input element at a point on the element
    /// that also passes a filter, in the element's coordinates.
    ///
    /// If `filter` returns false for a visual then the visual and all its
    /// descendants are excluded from the results. `enabled_elements_only`
    /// tells whether only enabled elements should be considered.
    pub fn input_hit_test_filtered(
        &self,
        p: Point,
        filter: &dyn Fn(&Visual) -> bool,
        enabled_elements_only: bool,
    ) -> Option<Ref<InputElement>> {
        let hit_test_delegate = hit_test_filter(enabled_elements_only);

        self.get_visual_at_filtered(p, &|x: &Visual| hit_test_delegate(x) && filter(x))
            .and_then(|visual| visual.downcast::<InputElement>().ok())
    }

    /// Returns the topmost active input element intersecting a geometry on
    /// the element that also passes a filter, in the element's coordinates.
    ///
    /// If `filter` returns false for a visual then the visual and all its
    /// descendants are excluded from the results. `enabled_elements_only`
    /// tells whether only enabled elements should be considered.
    pub fn input_hit_test_geometry_filtered(
        &self,
        geometry: &Geometry,
        filter: &dyn Fn(&Visual) -> bool,
        enabled_elements_only: bool,
    ) -> Option<Ref<InputElement>> {
        let hit_test_delegate = hit_test_filter(enabled_elements_only);

        self.get_visual_at_geometry_filtered(geometry, &|x: &Visual| hit_test_delegate(x) && filter(x))
            .and_then(|hit_test| hit_test.visual_hit.downcast::<InputElement>().ok())
    }
}
