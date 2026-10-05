use super::ServerCompositionVisual;
use crate::platform::LtrbRect;
use crate::rendering::composition::generated::{CompositionVisualChangedFields as F, ServerCompositionVisualProps as P};
use crate::rendering::composition::server::CompositionProperty;

const COMPOSITION_FIELDS_MASK: F = F::OPACITY
    .union(F::OPACITY_ANIMATED)
    .union(F::OPACITY_MASK_BRUSH)
    .union(F::CLIP)
    .union(F::CLIP_TO_BOUNDS)
    .union(F::CLIP_TO_BOUNDS_ANIMATED)
    .union(F::SIZE)
    .union(F::SIZE_ANIMATED)
    .union(F::RENDER_OPTIONS)
    .union(F::EFFECT);

const OWN_BOUNDS_UPDATE_FIELDS_MASK: F = F::CLIP
    .union(F::CLIP_TO_BOUNDS)
    .union(F::CLIP_TO_BOUNDS_ANIMATED)
    .union(F::SIZE)
    .union(F::SIZE_ANIMATED)
    .union(F::EFFECT);

const COMBINED_TRANSFORM_FIELDS_MASK: F = F::SIZE
    .union(F::SIZE_ANIMATED)
    .union(F::ANCHOR_POINT)
    .union(F::ANCHOR_POINT_ANIMATED)
    .union(F::CENTER_POINT)
    .union(F::CENTER_POINT_ANIMATED)
    .union(F::ADORNED_VISUAL)
    .union(F::TRANSFORM_MATRIX)
    .union(F::SCALE)
    .union(F::SCALE_ANIMATED)
    .union(F::ROTATION_ANGLE)
    .union(F::ROTATION_ANGLE_ANIMATED)
    .union(F::ORIENTATION)
    .union(F::ORIENTATION_ANIMATED)
    .union(F::OFFSET)
    .union(F::OFFSET_ANIMATED)
    .union(F::TRANSLATION)
    .union(F::TRANSLATION_ANIMATED);

const CLIP_SIZE_DIRTY_MASK: F =
    F::SIZE.union(F::SIZE_ANIMATED).union(F::CLIP_TO_BOUNDS).union(F::CLIP).union(F::CLIP_TO_BOUNDS_ANIMATED);

const READBACK_DIRTY_MASK: F =
    COMBINED_TRANSFORM_FIELDS_MASK.union(F::ROOT).union(F::VISIBLE).union(F::VISIBLE_ANIMATED);

impl ServerCompositionVisual {
    pub(super) fn on_fields_deserialized_core(&self, changed: F) {
        if changed.intersects(COMPOSITION_FIELDS_MASK) {
            self.trigger_composition_fields_dirty();
        }
        if changed.intersects(COMBINED_TRANSFORM_FIELDS_MASK) {
            self.trigger_combined_transform_dirty();
        }
        if changed.intersects(CLIP_SIZE_DIRTY_MASK) {
            self.trigger_clip_size_dirty();
        }

        if changed.intersects(OWN_BOUNDS_UPDATE_FIELDS_MASK) {
            self.own_bounds_dirty.set(true);
            self.enqueue_own_properties_recompute();
        }

        if changed.intersects(READBACK_DIRTY_MASK) {
            self.enqueue_for_readback_update();
        }

        if changed.intersects(F::VISIBLE | F::VISIBLE_ANIMATED) {
            self.trigger_visible_dirty();
        }

        if changed.intersects(F::SIZE_ANIMATED | F::SIZE) {
            self.content.size_changed(self);
        }
    }

    pub(super) fn notify_animated_value_changed_core(&self, property: &'static CompositionProperty) {
        // base.NotifyAnimatedValueChanged: ValuesInvalidated() does nothing
        // for a visual.
        let is = |p: &CompositionProperty| p == property;
        let clip_to_bounds = is(P::id_of_clip_to_bounds_property().base());
        let size = is(P::id_of_size_property().base());

        if clip_to_bounds || is(P::id_of_opacity_property().base()) || size {
            self.trigger_composition_fields_dirty();
        }

        if size
            || is(P::id_of_anchor_point_property().base())
            || is(P::id_of_center_point_property().base())
            || is(P::id_of_adorned_visual_property().base())
            || is(P::id_of_transform_matrix_property().base())
            || is(P::id_of_scale_property().base())
            || is(P::id_of_rotation_angle_property().base())
            || is(P::id_of_orientation_property().base())
            || is(P::id_of_offset_property().base())
            || is(P::id_of_translation_property().base())
        {
            self.trigger_combined_transform_dirty();
        }

        if clip_to_bounds || size {
            self.trigger_clip_size_dirty();
        }

        if size {
            self.content.size_changed(self);
        }

        if is(P::id_of_visible_property().base()) {
            self.trigger_visible_dirty();
        }
    }

    pub(crate) fn trigger_composition_fields_dirty(&self) {
        self.composition_fields_dirty.set(true);
        self.enqueue_own_properties_recompute();
    }

    pub(crate) fn trigger_combined_transform_dirty(&self) {
        self.combined_transform_dirty.set(true);
        self.enqueue_own_properties_recompute();
        self.enqueue_for_readback_update();
    }

    pub(crate) fn trigger_clip_size_dirty(&self) {
        self.enqueue_own_properties_recompute();
        self.clip_size_dirty.set(true);
    }

    pub(crate) fn trigger_visible_dirty(&self) {
        self.enqueue_for_readback_update();
        self.enqueue_for_own_bounds_recompute();
    }

    pub(super) fn on_parent_changing_core(&self) {
        if let (Some(parent), Some(bounds)) = (self.parent(), self.transformed_sub_tree_bounds.get()) {
            parent.add_extra_dirty_rect(bounds);
        }
        self.att_helper_parent_changing();
    }

    pub(super) fn on_parent_changed_core(&self) {
        if self.parent().is_some() {
            self.delay_propagate_needs_bounds_update.set(true);
            self.delay_propagate_is_dirty_for_render.set(true);
            self.enqueue_own_properties_recompute();
        }
        self.att_helper_parent_changed();
    }

    pub(crate) fn add_extra_dirty_rect(&self, rect: LtrbRect) {
        self.extra_dirty_rect.set(if self.delay_propagate_has_extra_dirty_rects.get() {
            self.extra_dirty_rect.get().union(rect)
        } else {
            rect
        });
        self.delay_propagate_has_extra_dirty_rects.set(true);
        self.enqueue_own_properties_recompute();
    }

    pub(crate) fn enqueue_for_own_bounds_recompute(&self) {
        self.own_bounds_dirty.set(true);
        self.enqueue_own_properties_recompute();
    }

    pub(crate) fn invalidate_content(&self) {
        self.content_changed.set(true);
        self.enqueue_for_own_bounds_recompute();
    }

    fn enqueue_own_properties_recompute(&self) {
        if self.enqueued_for_own_properties_recompute.get() {
            return;
        }
        if let (Some(compositor), Some(this)) = (self.compositor(), self.rc()) {
            self.enqueued_for_own_properties_recompute.set(true);
            compositor.enqueue_visual_for_own_properties_update_pass(this);
        }
    }
}
