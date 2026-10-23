//! The WinUI members of `Visual` on the server (phase 2, not from upstream):
//! the rotation axis, the relative offset and size adjustments, pixel
//! snapping and `ParentForTransform`, with the bookkeeping that keeps them
//! up to date.

use super::act::ActSubscriber;
use super::ServerCompositionVisual;
use crate::media::EffectExtensions;
use crate::numerics::{Quaternion, Vector3};
use crate::platform::LtrbRect;
use crate::rendering::composition::generated::ServerCompositionVisualProps;
use crate::rendering::composition::{
    CompositionBackfaceVisibility, CompositionBorderMode, CompositionCompositeMode, MatrixUtils,
};
use crate::{Matrix, Vector, Vector3D};
use std::rc::Rc;

impl ServerCompositionVisual {
    // --- the properties, typed ------------------------------------------------

    /// The axis `RotationAngle` rotates about.
    pub fn rotation_axis(&self) -> Vector3D {
        self.props.base().rotation_axis()
    }

    /// The part of the size of the parent added to the offset.
    pub fn relative_offset_adjustment(&self) -> Vector3D {
        self.props.base().relative_offset_adjustment()
    }

    /// The part of the size of the parent added to the size.
    pub fn relative_size_adjustment(&self) -> Vector {
        self.props.base().relative_size_adjustment()
    }

    pub fn is_hit_test_visible(&self) -> bool {
        self.props.base().is_hit_test_visible()
    }

    pub fn is_pixel_snapping_enabled(&self) -> bool {
        self.props.base().is_pixel_snapping_enabled()
    }

    pub fn border_mode(&self) -> CompositionBorderMode {
        self.props.base().border_mode()
    }

    pub fn backface_visibility(&self) -> CompositionBackfaceVisibility {
        self.props.base().backface_visibility()
    }

    pub fn composite_mode(&self) -> CompositionCompositeMode {
        self.props.base().composite_mode()
    }

    /// The visual whose coordinate space the transform of this visual is
    /// relative to.
    pub fn parent_for_transform(&self) -> Option<Rc<ServerCompositionVisual>> {
        self.props
            .base()
            .parent_for_transform()
            .and_then(|visual| visual.into_any_rc().downcast::<ServerCompositionVisual>().ok())
    }

    // --- effective size and offset ---------------------------------------------

    /// The size of the visual: `Size` plus `RelativeSizeAdjustment` times the
    /// size of the parent.
    pub(super) fn effective_size(&self) -> Vector {
        let size = self.props.base().size();
        let relative = self.relative_size_adjustment();
        if relative == Vector::default() {
            return size;
        }
        let parent = self.parent().map(|parent| parent.size()).unwrap_or_default();
        size + Vector::multiply(relative, parent)
    }

    /// The offset of the visual: `Offset` plus `RelativeOffsetAdjustment`
    /// times the size of the parent (X and Y; the parent has no depth).
    pub(super) fn effective_offset(&self) -> Vector3D {
        let offset = self.props.base().offset();
        let relative = self.relative_offset_adjustment();
        if relative.x == 0.0 && relative.y == 0.0 {
            return offset;
        }
        let parent = self.parent().map(|parent| parent.size()).unwrap_or_default();
        offset + Vector3D::new(relative.x * parent.x, relative.y * parent.y, 0.0)
    }

    /// Whether the offset or the size of the visual follows the size of its
    /// parent.
    fn follows_parent_size(&self) -> (bool, bool) {
        let offset = self.relative_offset_adjustment();
        (offset.x != 0.0 || offset.y != 0.0, self.relative_size_adjustment() != Vector::default())
    }

    /// The size of the visual changed (its `Size`, its relative size
    /// adjustment or the size of the parent it follows): the children whose
    /// offset or size follow it are updated.
    pub(super) fn notify_children_of_size_change(&self) {
        let Some(children) = self.children() else { return };
        for child in children.list().iter() {
            child.parent_size_changed();
        }
    }

    /// The size of the parent changed, or the visual has a new parent.
    pub(super) fn parent_size_changed(&self) {
        let (offset_follows, size_follows) = self.follows_parent_size();
        if size_follows {
            self.effective_size_changed();
        } else if offset_follows {
            self.trigger_combined_transform_dirty();
        }
    }

    /// What a change of `Size` does, for a change of the effective size that
    /// does not come from the property.
    fn effective_size_changed(&self) {
        self.trigger_composition_fields_dirty();
        self.trigger_combined_transform_dirty();
        self.trigger_clip_size_dirty();
        self.own_bounds_dirty.set(true);
        self.content.size_changed(self);
        self.notify_children_of_size_change();
    }

    // --- the transform ------------------------------------------------------------

    /// The rotation of the visual as the 2D transform takes it: the Z rotation
    /// and the orientation. A `RotationAxis` along Z (or zero) rotates in the
    /// plane; any other axis is folded into the orientation, applied before
    /// `Orientation`.
    fn effective_rotation(&self) -> (f32, Quaternion) {
        let angle = self.rotation_angle();
        let orientation = self.orientation();
        let axis = self.rotation_axis();
        if angle == 0.0 || (axis.x == 0.0 && axis.y == 0.0) {
            let angle = if axis.z < 0.0 { -angle } else { angle };
            return (angle, orientation);
        }
        let axis = Vector3::normalize(axis.into());
        let rotation = Quaternion::create_from_axis_angle(axis, angle);
        (0.0, Quaternion::concatenate(rotation, orientation))
    }

    /// The transform of the visual from its properties, relative to its
    /// parent: what the own-properties pass computes, with the WinUI members
    /// applied (the relative adjustments, the rotation axis, pixel snapping).
    pub(super) fn compute_own_transform_from_properties(&self, transform_matrix: Matrix) -> Option<Matrix> {
        let (rotation_angle, orientation) = self.effective_rotation();
        let transform = MatrixUtils::compute_transform(
            self.size(),
            self.anchor_point(),
            self.center_point(),
            transform_matrix,
            self.scale(),
            rotation_angle,
            orientation,
            self.offset() + self.translation(),
        );
        if !self.is_pixel_snapping_enabled() {
            return transform;
        }
        // Deviation (winui-composition.md): the translation relative to the
        // parent is rounded to the device pixels of the target.
        let scaling = self.root().map(|root| root.scaling()).filter(|scaling| *scaling > 0.0).unwrap_or(1.0);
        let mut snapped = transform.unwrap_or(Matrix::IDENTITY);
        snapped.m31 = (snapped.m31 * scaling).round() / scaling;
        snapped.m32 = (snapped.m32 * scaling).round() / scaling;
        (!snapped.is_identity()).then_some(snapped)
    }

    /// The transform from the space of `ParentForTransform` to the space of
    /// the parent; `None` when there is no such visual or the two are not in
    /// one tree.
    fn parent_for_transform_matrix(&self) -> Option<Matrix> {
        let target = self.parent_for_transform()?;
        let parent = self.parent()?;
        if Rc::ptr_eq(&target, &parent) {
            return Some(Matrix::IDENTITY);
        }
        let mut target_chain = Vec::new();
        let mut visual = Some(target.clone());
        while let Some(current) = visual {
            visual = current.parent();
            target_chain.push(current);
        }
        let mut candidate = Some(parent.clone());
        while let Some(ancestor) = candidate {
            if target_chain.iter().any(|visual| Rc::ptr_eq(visual, &ancestor)) {
                let target_to_ancestor = if Rc::ptr_eq(&target, &ancestor) {
                    Matrix::IDENTITY
                } else {
                    Self::compute_transform_from_ancestor(&target, &ancestor)?
                };
                let parent_to_ancestor = if Rc::ptr_eq(&parent, &ancestor) {
                    Matrix::IDENTITY
                } else {
                    Self::compute_transform_from_ancestor(&parent, &ancestor)?
                };
                return Some(target_to_ancestor * parent_to_ancestor.try_invert()?);
            }
            candidate = ancestor.parent();
        }
        None
    }

    /// The own transform of a visual with a `ParentForTransform`, computed by
    /// the adorner pass (after the own-properties pass, when the transforms of
    /// the other visuals are known).
    pub(super) fn update_parent_for_transform(&self) {
        let own = self.compute_own_transform_from_properties(self.transform_matrix());
        let transform = match self.parent_for_transform_matrix() {
            Some(relative) => {
                let transform = own.unwrap_or(Matrix::IDENTITY) * relative;
                (!transform.is_identity()).then_some(transform)
            }
            None => own,
        };
        self.own_transform.set(transform);
    }

    fn parent_for_transform_subscription(&self) -> ActSubscriber {
        self.adorned_visual_act_subscription_action()
    }

    /// Follows the world transforms of `ParentForTransform` and of the parent:
    /// a change of either enqueues the visual for the adorner pass.
    fn subscribe_parent_for_transform(&self, subscribe: bool) {
        let Some(target) = self.parent_for_transform() else { return };
        let subscription = self.parent_for_transform_subscription();
        let mut followed = vec![target];
        if let Some(parent) = self.parent() {
            followed.push(parent);
        }
        for visual in followed {
            if visual.disposed.get() {
                continue;
            }
            if subscribe {
                visual.att_helper_subscribe_to_act_notification(subscription.clone());
            } else {
                visual.att_helper_unsubscribe_from_act_notification(&subscription);
            }
        }
    }

    pub(super) fn on_parent_for_transform_changing_core(&self) {
        self.subscribe_parent_for_transform(false);
    }

    pub(super) fn on_parent_for_transform_changed_core(&self) {
        self.subscribe_parent_for_transform(true);
        self.trigger_combined_transform_dirty();
        self.adorner_helper_enqueue_for_adorner_update();
    }

    /// The parent is about to change: the subscription to the old parent goes.
    pub(super) fn winui_members_parent_changing(&self) {
        if self.parent_for_transform().is_some() {
            if let Some(parent) = self.parent() {
                parent.att_helper_unsubscribe_from_act_notification(&self.parent_for_transform_subscription());
            }
        }
    }

    /// The parent changed: the offset and size follow the new parent, and the
    /// transform relative to `ParentForTransform` is computed again.
    pub(super) fn winui_members_parent_changed(&self) {
        if self.parent_for_transform().is_some() {
            if let Some(parent) = self.parent() {
                parent.att_helper_subscribe_to_act_notification(self.parent_for_transform_subscription());
            }
            self.adorner_helper_enqueue_for_adorner_update();
        }
        if self.parent().is_some() {
            self.parent_size_changed();
        }
    }

    /// The visual is disposed: the subscriptions go and the reference to
    /// `ParentForTransform` is dropped without notifications.
    pub(super) fn winui_members_dispose(&self) {
        self.subscribe_parent_for_transform(false);
        ServerCompositionVisualProps::id_of_parent_for_transform_property().set_field(self, None);
    }

    // --- bounds -------------------------------------------------------------------

    /// The bounds of the content and the children of the visual in its own
    /// coordinates, with the padding of its effect, ignoring its own clip and
    /// visibility: what a visual surface or a redirect visual that shows the
    /// visual covers (not from upstream). As of the last update of the target
    /// of the visual.
    pub fn unclipped_sub_tree_bounds(&self) -> Option<LtrbRect> {
        let mut bounds = self.own_content_bounds.get();
        if let Some(children) = self.children() {
            for child in children.list().iter() {
                bounds = LtrbRect::full_union(bounds, child.transformed_sub_tree_bounds.get());
            }
        }
        match (bounds, self.effect()) {
            (Some(bounds), Some(effect)) => {
                Some(bounds.inflate(EffectExtensions::get_effect_output_padding(Some(&*effect))))
            }
            (bounds, _) => bounds,
        }
    }

    /// Whether `ancestor` is this visual or one of its ancestors.
    pub fn is_self_or_descendant_of(&self, ancestor: &ServerCompositionVisual) -> bool {
        if std::ptr::eq(self, ancestor) {
            return true;
        }
        let mut visual = self.parent();
        while let Some(current) = visual {
            if std::ptr::eq(&*current, ancestor) {
                return true;
            }
            visual = current.parent();
        }
        false
    }
}
