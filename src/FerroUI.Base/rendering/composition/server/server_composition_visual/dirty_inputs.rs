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
    .union(F::EFFECT)
    .union(F::COMPOSITION_CLIP)
    .union(WINUI_SIZE_FIELDS_MASK);

/// The WinUI size adjustment (not from upstream): it changes the size of the
/// visual as `Size` does.
const WINUI_SIZE_FIELDS_MASK: F = F::RELATIVE_SIZE_ADJUSTMENT.union(F::RELATIVE_SIZE_ADJUSTMENT_ANIMATED);

/// The WinUI members that change the transform of the visual (not from
/// upstream).
const WINUI_TRANSFORM_FIELDS_MASK: F = F::ROTATION_AXIS
    .union(F::ROTATION_AXIS_ANIMATED)
    .union(F::RELATIVE_OFFSET_ADJUSTMENT)
    .union(F::RELATIVE_OFFSET_ADJUSTMENT_ANIMATED)
    .union(F::IS_PIXEL_SNAPPING_ENABLED)
    .union(F::PARENT_FOR_TRANSFORM)
    .union(WINUI_SIZE_FIELDS_MASK);

const OWN_BOUNDS_UPDATE_FIELDS_MASK: F = F::CLIP
    .union(F::CLIP_TO_BOUNDS)
    .union(F::CLIP_TO_BOUNDS_ANIMATED)
    .union(F::SIZE)
    .union(F::SIZE_ANIMATED)
    .union(F::EFFECT)
    .union(F::COMPOSITION_CLIP)
    .union(WINUI_SIZE_FIELDS_MASK);

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
    .union(F::TRANSLATION_ANIMATED)
    .union(WINUI_TRANSFORM_FIELDS_MASK);

const CLIP_SIZE_DIRTY_MASK: F = F::SIZE
    .union(F::SIZE_ANIMATED)
    .union(F::CLIP_TO_BOUNDS)
    .union(F::CLIP)
    .union(F::CLIP_TO_BOUNDS_ANIMATED)
    .union(F::COMPOSITION_CLIP)
    .union(WINUI_SIZE_FIELDS_MASK);

/// The content has drawing items: the render walk looks for them between the children.
const HAS_DRAW_ITEMS: u8 = 1;
/// Something of the drawing items changed since the update pass last took their changes.
const DRAW_ITEMS_DIRTY: u8 = 2;

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

        if changed.intersects((F::SIZE_ANIMATED | F::SIZE).union(WINUI_SIZE_FIELDS_MASK)) {
            self.content.size_changed(self);
            // The children whose offset or size follow it (WinUI).
            self.notify_children_of_size_change();
        }
    }

    pub(super) fn notify_animated_value_changed_core(&self, property: &'static CompositionProperty) {
        // base.NotifyAnimatedValueChanged: ValuesInvalidated() does nothing
        // for a visual.
        let is = |p: &CompositionProperty| p == property;
        let clip_to_bounds = is(P::id_of_clip_to_bounds_property().base());
        // The WinUI size adjustment changes the size as `Size` does.
        let size = is(P::id_of_size_property().base()) || is(P::id_of_relative_size_adjustment_property().base());

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
            || is(P::id_of_rotation_axis_property().base())
            || is(P::id_of_relative_offset_adjustment_property().base())
        {
            self.trigger_combined_transform_dirty();
        }

        if clip_to_bounds || size {
            self.trigger_clip_size_dirty();
        }

        if size {
            self.content.size_changed(self);
            self.notify_children_of_size_change();
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
        if let Some(parent) = self.parent() {
            // Not upstream, which leaves the bounds on the visual: they are in the space of the
            // parent the visual leaves, and that parent redraws them. Under its next parent, or
            // under the same one frames later, they are not where the visual is drawn, and the
            // update pass would redraw that place for nothing (docs/porting/
            // desktop-performance.md, section 9.32).
            if let Some(bounds) = self.transformed_sub_tree_bounds.take() {
                parent.add_extra_dirty_rect(bounds);
            }
        }
        self.att_helper_parent_changing();
        self.winui_members_parent_changing();
    }

    pub(super) fn on_parent_changed_core(&self) {
        if self.parent().is_some() {
            self.delay_propagate_needs_bounds_update.set(true);
            self.delay_propagate_is_dirty_for_render.set(true);
            self.enqueue_own_properties_recompute();
        }
        self.att_helper_parent_changed();
        self.winui_members_parent_changed();
    }

    pub(crate) fn add_extra_dirty_rect(&self, rect: LtrbRect) {
        // A rect of zero size is what a change of the drawing items left there: it has
        // nothing to unite with.
        let current = self.extra_dirty_rect.get();
        self.extra_dirty_rect.set(if self.delay_propagate_has_extra_dirty_rects.get() && !current.is_zero_size() {
            current.union(rect)
        } else {
            rect
        });
        self.delay_propagate_has_extra_dirty_rects.set(true);
        self.enqueue_own_properties_recompute();
    }

    /// Whether the content has drawing items.
    #[inline]
    pub(crate) fn has_draw_items(&self) -> bool {
        self.draw_item_flags.get() & HAS_DRAW_ITEMS != 0
    }

    pub(crate) fn set_has_draw_items(&self, value: bool) {
        let flags = self.draw_item_flags.get();
        self.draw_item_flags.set(if value { flags | HAS_DRAW_ITEMS } else { flags & !HAS_DRAW_ITEMS });
    }

    /// Whether the drawing items changed since the last call; the update pass asks.
    pub(super) fn take_draw_items_dirty(&self) -> bool {
        let flags = self.draw_item_flags.get();
        self.draw_item_flags.set(flags & !DRAW_ITEMS_DIRTY);
        flags & DRAW_ITEMS_DIRTY != 0
    }

    pub(super) fn draw_items_dirty(&self) -> bool {
        self.draw_item_flags.get() & DRAW_ITEMS_DIRTY != 0
    }

    /// Something of the drawing items of the content changed: the update pass adds where
    /// the changed items were and are to the dirty region, as the extra dirty rects of the
    /// visual, and the bounds of the content are computed again. The visual is not dirty for
    /// render: what else it shows is not drawn again for an item (section 9.42).
    pub(crate) fn content_items_changed(&self) {
        self.draw_item_flags.set(self.draw_item_flags.get() | DRAW_ITEMS_DIRTY);
        if !self.delay_propagate_has_extra_dirty_rects.get() {
            // A rect the update pass has not taken yet stays; one it took is not added again.
            if !self.needs_to_add_extra_dirty_rect_to_dirty_region.get() {
                self.extra_dirty_rect.set(LtrbRect::default());
            }
            self.delay_propagate_has_extra_dirty_rects.set(true);
        }
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
