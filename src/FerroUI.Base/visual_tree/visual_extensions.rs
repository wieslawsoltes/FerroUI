//! Extension methods for [`Visual`]: visual tree traversal, coordinate
//! conversion and hit testing.
//!
//! This file ports both the visual tree extensions and the coordinate
//! conversion extensions of the root namespace. Members that already exist
//! on [`Visual`] itself (`visual_parent`, `presentation_source`,
//! `visual_root`, `is_attached_to_visual_tree`) are not repeated here.

use super::TransformedBounds;
use crate::input::IInputRoot;
use crate::media::{Geometry, GeometryHitTestResult};
use crate::platform::IPlatformSettings;
use crate::{Matrix, ObjectType, PixelPoint, Point, Rect, Ref, Visual};
use std::rc::Rc;

/// The render transform of a visual, already adjusted for its transform
/// origin, or `None` when the visual has none.
///
/// This is the single place the visual tree helpers and the managed hit
/// tester get the render transform from.
#[inline]
fn render_transform(visual: &Visual) -> Option<Matrix> {
    visual.render_transform().map(|transform| {
        let origin = visual.render_transform_origin().to_pixels(visual.bounds().size());
        let offset = Matrix::create_translation(origin.x, origin.y);
        -offset * transform.value() * offset
    })
}

/// The transform of a visual relative to its parent, excluding the
/// translation to its position: the mirror transform (applied first) and
/// the render transform.
#[inline]
fn local_render_transform(visual: &Visual) -> Matrix {
    let mut result = Matrix::IDENTITY;

    // This should be calculated BEFORE the render transform.
    if visual.has_mirror_transform() {
        let mirror_matrix = Matrix::new(-1.0, 0.0, 0.0, 1.0, visual.bounds().width, 0.0);
        result *= mirror_matrix;
    }

    if let Some(render_transform) = render_transform(visual) {
        result *= render_transform;
    }

    result
}

/// The transform from the coordinate space of a visual to the coordinate
/// space of its parent.
pub(crate) fn local_transform(visual: &Visual) -> Matrix {
    let mut result = local_render_transform(visual);
    let top_left = visual.bounds().top_left();

    if top_left != Point::default() {
        result *= Matrix::create_translation(top_left.x, top_left.y);
    }

    result
}

/// Iterates the visual ancestors of a visual, from its parent to the root.
pub struct VisualAncestors {
    next: Option<Ref<Visual>>,
}

impl Iterator for VisualAncestors {
    type Item = Ref<Visual>;

    fn next(&mut self) -> Option<Ref<Visual>> {
        let current = self.next.take()?;
        self.next = current.visual_parent();
        Some(current)
    }
}

/// Iterates the visual descendants of a visual in depth-first pre-order.
pub struct VisualDescendants {
    /// The snapshots of the children collections being enumerated, with the
    /// index of the next child to visit in each.
    stack: Vec<(Rc<Vec<Ref<Visual>>>, usize)>,
    pending: Option<Ref<Visual>>,
}

impl Iterator for VisualDescendants {
    type Item = Ref<Visual>;

    fn next(&mut self) -> Option<Ref<Visual>> {
        if let Some(pending) = self.pending.take() {
            return Some(pending);
        }

        loop {
            let (children, index) = self.stack.last_mut()?;
            if *index >= children.len() {
                self.stack.pop();
                continue;
            }

            let child = children[*index].clone();
            *index += 1;

            if let Some(grandchildren) = child.visual_children_snapshot() {
                if !grandchildren.is_empty() {
                    self.stack.push((grandchildren, 0));
                }
            }

            return Some(child);
        }
    }
}

impl Visual {
    /// Calculates the distance from a visual's ancestor.
    ///
    /// Returns the number of steps from the visual to the ancestor, or
    /// `None` if this visual is not a descendent of `ancestor`. With no
    /// ancestor given, never matches and returns `None`.
    pub fn calculate_distance_from_ancestor(&self, ancestor: Option<&Visual>) -> Option<usize> {
        let mut result = 0;
        let mut v = Some(self.to_ref());

        while let Some(current) = v {
            if ancestor.is_some_and(|a| std::ptr::eq::<Visual>(&*current, a)) {
                return Some(result);
            }
            v = current.visual_parent();
            result += 1;
        }

        None
    }

    /// Calculates the distance from a visual's root.
    pub fn calculate_distance_from_root(&self) -> usize {
        let mut result = 0;
        let mut v = self.visual_parent();

        while let Some(current) = v {
            v = current.visual_parent();
            result += 1;
        }

        result
    }

    /// Tries to get the first common ancestor of two visuals.
    pub fn find_common_visual_ancestor(&self, target: &Visual) -> Option<Ref<Visual>> {
        fn go_upwards(node: &mut Option<Ref<Visual>>, count: usize) {
            for _ in 0..count {
                *node = node.as_ref().and_then(|n| n.visual_parent());
            }
        }

        let mut v = Some(self.to_ref());
        let mut t = Some(target.to_ref());

        // We want to find the lowest node first, then make sure that both
        // nodes are at the same height. By doing that we can sometimes find
        // out that the other node is our lowest common ancestor.
        let first_height = self.calculate_distance_from_root();
        let second_height = target.calculate_distance_from_root();

        if first_height > second_height {
            go_upwards(&mut v, first_height - second_height);
        } else {
            go_upwards(&mut t, second_height - first_height);
        }

        if v == t {
            return v;
        }

        while let (Some(first), Some(second)) = (&v, &t) {
            let first_parent = first.visual_parent();
            let second_parent = second.visual_parent();

            if first_parent == second_parent {
                return first_parent;
            }

            v = first_parent;
            t = second_parent;
        }

        None
    }

    /// Enumerates the ancestors of the visual in the visual tree.
    pub fn get_visual_ancestors(&self) -> VisualAncestors {
        VisualAncestors { next: self.visual_parent() }
    }

    /// Enumerates the visual and its ancestors in the visual tree.
    pub fn get_self_and_visual_ancestors(&self) -> VisualAncestors {
        VisualAncestors { next: Some(self.to_ref()) }
    }

    /// Finds the first ancestor of class `T`, optionally starting with the
    /// visual itself.
    pub fn find_ancestor_of_type<T: ObjectType>(&self, include_self: bool) -> Option<Ref<T>> {
        self.find_ancestor_of_type_where::<T>(include_self, |_| true)
    }

    /// Finds the first ancestor of class `T` that matches a predicate,
    /// optionally starting with the visual itself.
    pub fn find_ancestor_of_type_where<T: ObjectType>(
        &self,
        include_self: bool,
        predicate: impl Fn(&T) -> bool,
    ) -> Option<Ref<T>> {
        let mut parent = if include_self { Some(self.to_ref()) } else { self.visual_parent() };

        while let Some(current) = parent {
            if let Some(result) = current.downcast_ref::<T>() {
                if predicate(result) {
                    return current.downcast::<T>().ok();
                }
            }
            parent = current.visual_parent();
        }

        None
    }

    /// Finds the first descendant of class `T` (depth first), optionally
    /// starting with the visual itself.
    pub fn find_descendant_of_type<T: ObjectType>(&self, include_self: bool) -> Option<Ref<T>> {
        self.find_descendant_of_type_where::<T>(include_self, |_| true)
    }

    /// Finds the first descendant of class `T` that matches a predicate
    /// (depth first), optionally starting with the visual itself.
    pub fn find_descendant_of_type_where<T: ObjectType>(
        &self,
        include_self: bool,
        predicate: impl Fn(&T) -> bool,
    ) -> Option<Ref<T>> {
        fn core<T: ObjectType>(visual: &Visual, predicate: &dyn Fn(&T) -> bool) -> Option<Ref<T>> {
            let children = visual.visual_children_snapshot()?;

            for child in children.iter() {
                if let Some(result) = child.downcast_ref::<T>() {
                    if predicate(result) {
                        return child.cast::<T>();
                    }
                }

                if let Some(child_result) = core::<T>(child, predicate) {
                    return Some(child_result);
                }
            }

            None
        }

        if include_self {
            if let Some(result) = self.downcast_ref::<T>() {
                if predicate(result) {
                    return self.to_ref().downcast::<T>().ok();
                }
            }
        }

        core::<T>(self, &predicate)
    }

    /// Gets the bounds of the visual in the coordinate space of its root,
    /// with the clip applied by its ancestors. Returns `None` if the visual
    /// or one of its ancestors is invisible.
    pub fn get_transformed_bounds(&self) -> Option<TransformedBounds> {
        fn visit(visual: &Visual, clip: &mut Rect, transform: &mut Matrix) -> bool {
            if !visual.is_visible() {
                return false;
            }

            // The visual's bounds in local coordinates.
            let bounds = Rect::from_size(visual.bounds().size());

            // If the visual has no parent, we've reached the root. We start
            // the clip rectangle with these bounds.
            let Some(parent) = visual.visual_parent() else {
                *clip = bounds;
                return true;
            };

            // Otherwise recurse until the root visual is found, exiting
            // early if one of the ancestors is invisible.
            if !visit(&parent, clip, transform) {
                return false;
            }

            // Calculate the transform for this control from its offset and
            // render transform.
            let render_transform = local_render_transform(visual);
            let position = visual.bounds().position();

            *transform = render_transform * Matrix::create_translation(position.x, position.y) * *transform;

            // If the visual is clipped, update the clip bounds.
            if visual.clip_to_bounds() {
                let global_bounds = bounds.transform_to_aabb(*transform);
                let clip_bounds = global_bounds.intersect(*clip);
                *clip = clip.intersect(clip_bounds);
            }

            true
        }

        let mut clip = Rect::default();
        let mut transform = Matrix::IDENTITY;

        if visit(self, &mut clip, &mut transform) {
            Some(TransformedBounds::new(Rect::from_size(self.bounds().size()), clip, transform))
        } else {
            None
        }
    }

    /// Gets the first visible visual at a point, in the coordinates of this
    /// visual.
    pub fn get_visual_at(&self, p: Point) -> Option<Ref<Visual>> {
        self.get_visual_at_filtered(p, &|x: &Visual| x.is_visible())
    }

    /// Gets the first visual at a point, in the coordinates of this visual.
    ///
    /// If `filter` returns false for a visual then the visual and all its
    /// descendants are excluded from the results.
    pub fn get_visual_at_filtered(&self, p: Point, filter: &dyn Fn(&Visual) -> bool) -> Option<Ref<Visual>> {
        let source = self.presentation_source()?;
        source.hit_tester().hit_test_first(p, self, Some(filter))
    }

    /// Gets the first visible visual intersecting a geometry, in the
    /// coordinates of this visual, with the details of the intersection.
    pub fn get_visual_at_geometry(&self, geometry: &Geometry) -> Option<GeometryHitTestResult> {
        self.get_visual_at_geometry_filtered(geometry, &|x: &Visual| x.is_visible())
    }

    /// Gets the first visual intersecting a geometry, in the coordinates of
    /// this visual, with the details of the intersection.
    ///
    /// If `filter` returns false for a visual then the visual and all its
    /// descendants are excluded from the results.
    pub fn get_visual_at_geometry_filtered(
        &self,
        geometry: &Geometry,
        filter: &dyn Fn(&Visual) -> bool,
    ) -> Option<GeometryHitTestResult> {
        let source = self.presentation_source()?;
        source.hit_tester().hit_test_first_geometry(geometry, self, Some(filter))
    }

    /// Enumerates the visible visuals at a point, in the coordinates of
    /// this visual, topmost first.
    pub fn get_visuals_at(&self, p: Point) -> Vec<Ref<Visual>> {
        self.get_visuals_at_filtered(p, &|x: &Visual| x.is_visible())
    }

    /// Enumerates the visuals at a point, in the coordinates of this
    /// visual, topmost first.
    ///
    /// If `filter` returns false for a visual then the visual and all its
    /// descendants are excluded from the results.
    pub fn get_visuals_at_filtered(&self, p: Point, filter: &dyn Fn(&Visual) -> bool) -> Vec<Ref<Visual>> {
        match self.presentation_source() {
            Some(source) => source.hit_tester().hit_test(p, self, Some(filter)),
            None => Vec::new(),
        }
    }

    /// Enumerates the visible visuals intersecting a geometry, in the
    /// coordinates of this visual, topmost first, with the details of the
    /// intersections.
    pub fn get_visuals_at_geometry(&self, geometry: &Geometry) -> Vec<GeometryHitTestResult> {
        self.get_visuals_at_geometry_filtered(geometry, &|x: &Visual| x.is_visible())
    }

    /// Enumerates the visuals intersecting a geometry, in the coordinates
    /// of this visual, topmost first, with the details of the
    /// intersections.
    ///
    /// If `filter` returns false for a visual then the visual and all its
    /// descendants are excluded from the results.
    pub fn get_visuals_at_geometry_filtered(
        &self,
        geometry: &Geometry,
        filter: &dyn Fn(&Visual) -> bool,
    ) -> Vec<GeometryHitTestResult> {
        match self.presentation_source() {
            Some(source) => source.hit_tester().hit_test_geometry(geometry, self, Some(filter)),
            None => Vec::new(),
        }
    }

    /// Gets a snapshot of the visual children of the visual.
    pub fn get_visual_children(&self) -> Rc<Vec<Ref<Visual>>> {
        self.visual_children_snapshot().unwrap_or_default()
    }

    /// Enumerates the descendants of the visual in the visual tree, in
    /// depth-first pre-order.
    pub fn get_visual_descendants(&self) -> VisualDescendants {
        let mut stack = Vec::new();
        if let Some(children) = self.visual_children_snapshot() {
            if !children.is_empty() {
                stack.push((children, 0));
            }
        }
        VisualDescendants { stack, pending: None }
    }

    /// Enumerates the visual and its descendants in the visual tree, in
    /// depth-first pre-order.
    pub fn get_self_and_visual_descendants(&self) -> VisualDescendants {
        let mut result = self.get_visual_descendants();
        result.pending = Some(self.to_ref());
        result
    }

    /// Gets the visual parent of the visual.
    pub fn get_visual_parent(&self) -> Option<Ref<Visual>> {
        self.visual_parent()
    }

    /// Gets the visual parent of the visual, if it is of class `T`.
    pub fn get_visual_parent_of_type<T: ObjectType>(&self) -> Option<Ref<T>> {
        self.visual_parent().and_then(|parent| parent.downcast::<T>().ok())
    }

    /// The input root of the tree the visual is attached to.
    pub fn get_input_root(&self) -> Option<Rc<dyn IInputRoot>> {
        self.presentation_source().map(|source| source.input_root())
    }

    /// The platform settings of the tree the visual is attached to.
    pub fn get_platform_settings(&self) -> Option<Rc<dyn IPlatformSettings>> {
        self.presentation_source().and_then(|source| source.platform_settings())
    }

    /// Tests whether this visual is an ancestor of `target`.
    pub fn is_visual_ancestor_of(&self, target: &Visual) -> bool {
        let mut current = target.visual_parent();

        while let Some(c) = current {
            if std::ptr::eq::<Visual>(&*c, self) {
                return true;
            }
            current = c.visual_parent();
        }

        false
    }

    /// Sorts visuals for hit testing: by descending Z index, and for equal
    /// Z indices by descending position in the input, so that the topmost
    /// visual comes first.
    pub fn sort_by_z_index(elements: &[Ref<Visual>]) -> Vec<Ref<Visual>> {
        let mut indexed: Vec<(i32, usize)> =
            elements.iter().enumerate().map(|(index, element)| (element.z_index(), index)).collect();
        indexed.sort_by(|x, y| y.0.cmp(&x.0).then(y.1.cmp(&x.1)));
        indexed.into_iter().map(|(_, index)| elements[index].clone()).collect()
    }

    // --- coordinate conversion ----------------------------------------------

    /// Converts a point from screen to client coordinates.
    ///
    /// Panics if the visual does not belong to a visual tree.
    pub fn point_to_client(&self, point: PixelPoint) -> Point {
        let (source, root) = self.source_and_root();
        let root_point = source.point_to_client(point).unwrap_or_default();
        root.translate_point(root_point, self).expect("the root is an ancestor of the visual")
    }

    /// Converts a point from client to screen coordinates.
    ///
    /// Panics if the visual does not belong to a visual tree.
    pub fn point_to_screen(&self, point: Point) -> PixelPoint {
        let (source, root) = self.source_and_root();
        let p = self.translate_point(point, &root).expect("the root is an ancestor of the visual");
        source.point_to_screen(p).unwrap_or_default()
    }

    /// Returns a transform that transforms the visual's coordinates into
    /// the coordinates of the specified `to` visual, or `None` if the
    /// visuals don't share a common ancestor.
    pub fn transform_to_visual(&self, to: &Visual) -> Option<Matrix> {
        let common = self.find_common_visual_ancestor(to)?;
        let this_offset = Self::get_offset_from(&common, self);
        let that_offset = Self::get_offset_from(&common, to);
        let that_offset_inverted = that_offset.try_invert()?;

        Some(that_offset_inverted * this_offset)
    }

    /// Translates a point relative to this visual to coordinates that are
    /// relative to the specified visual, or returns `None` if the visuals
    /// don't share a common ancestor.
    pub fn translate_point(&self, point: Point, relative_to: &Visual) -> Option<Point> {
        self.transform_to_visual(relative_to).map(|transform| point.transform(transform))
    }

    fn source_and_root(&self) -> (Rc<dyn crate::rendering::IPresentationSource>, Ref<Visual>) {
        let source = self.presentation_source();
        let root = source.as_ref().and_then(|s| s.root_visual());
        match (source, root) {
            (Some(source), Some(root)) => (source, root),
            _ => panic!("Visual does not belong to a visual tree."),
        }
    }

    /// Gets a transform from an ancestor to a descendent.
    fn get_offset_from(ancestor: &Visual, visual: &Visual) -> Matrix {
        let mut result = Matrix::IDENTITY;
        let mut v = visual.to_ref();

        while !std::ptr::eq::<Visual>(&*v, ancestor) {
            result *= local_transform(&v);

            v = match v.visual_parent() {
                Some(parent) => parent,
                None => panic!("'visual' is not a descendant of 'ancestor'."),
            };
        }

        result
    }
}
