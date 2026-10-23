use super::ServerCompositionVisual;
use crate::platform::LtrbRect;
use crate::Matrix;

impl ServerCompositionVisual {
    pub fn own_transform(&self) -> Option<Matrix> {
        self.own_transform.get()
    }

    pub fn sub_tree_bounds(&self) -> Option<LtrbRect> {
        self.sub_tree_bounds.get()
    }

    pub fn transformed_sub_tree_bounds(&self) -> Option<LtrbRect> {
        self.transformed_sub_tree_bounds.get()
    }

    pub fn compute_own_content_bounds(&self) -> Option<LtrbRect> {
        self.content.compute_own_content_bounds(self)
    }

    pub fn combined_transform_matrix(&self) -> Matrix {
        self.combined_transform_matrix.get()
    }

    /// Whether the visual or a visual of its subtree changed since the last
    /// update pass of its target: what a visual surface showing the subtree
    /// follows (not from upstream).
    pub fn has_pending_render_changes(&self) -> bool {
        self.is_dirty_for_render.get()
            || self.is_dirty_for_render_in_subgraph.get()
            || self.needs_bounding_box_update.get()
            || self.needs_to_add_extra_dirty_rect_to_dirty_region.get()
    }

    // WPF's cheatsheet
    //-----------------------------------------------------------------------------
    //  Node Operation   | NeedsToBe     | NeedsBBoxUpdate | HasNodeThat    | Visit
    //                   | AddedToDirty  | (parent chain)  | NeedsToBeAdded | child
    //                   | Region        |                 | ToDirtyRegion  |
    //                   |               |                 | (parent chain) |
    //=============================================================================
    //  Set transform    |   Y           |   Y             |   Y(N)
    //  -----------------+---------------+-----------------+-----------------------
    //  Set opacity      |   Y           |   N             |   Y(N)
    //  -----------------+---------------+-----------------+-----------------------
    //  Set clip         |   Y           |   Y             |   Y(N)
    //  -----------------+---------------+-----------------+-----------------------
    //  AttachRenderData |   Y           |   Y             |   Y(N)
    //  -----------------+---------------+-----------------+-----------------------
    //  FreeRenderData   |   Y           |   Y             |   Y(N)
    //  -----------------+---------------+-----------------+-----------------------
    //  InsertChild      |   N           |   Y             |   Y
    //                   |   Y(child)    |   N             |   Y(N)
    //  -----------------+---------------+-----------------+-----------------------
    //  InsertChildAt    |   N           |   Y             |   Y
    //                   |   Y(child)    |   N             |   Y(N)
    //  -----------------+---------------+-----------------+-----------------------
    //  ZOrderChild      |   N           |   N             |   Y
    //                   |   Y(child)    |   N             |   Y(N)
    //  -----------------+---------------+-----------------+-----------------------
    //  ReplaceChild     |   Y           |   Y             |   Y(N)
    //  -----------------+---------------+-----------------+-----------------------
    //  RemoveChild      |   Y           |   Y             |   Y(N)
    pub(super) fn propagate_flags(
        &self,
        needs_bounding_box_update: bool,
        dirty_for_render: bool,
        additional_dirty_region: bool,
    ) {
        if let Some(root) = self.root() {
            root.request_update();
        }

        let mut parent = self.parent();
        let set_is_dirty_for_render_in_subgraph = additional_dirty_region || dirty_for_render;
        while let Some(p) = parent {
            if !((needs_bounding_box_update && !p.needs_bounding_box_update.get())
                || (set_is_dirty_for_render_in_subgraph && !p.is_dirty_for_render_in_subgraph.get()))
            {
                break;
            }
            p.needs_bounding_box_update.set(p.needs_bounding_box_update.get() | needs_bounding_box_update);
            p.is_dirty_for_render_in_subgraph
                .set(p.is_dirty_for_render_in_subgraph.get() | set_is_dirty_for_render_in_subgraph);
            parent = p.parent();
        }

        self.needs_bounding_box_update.set(self.needs_bounding_box_update.get() | needs_bounding_box_update);
        self.is_dirty_for_render.set(self.is_dirty_for_render.get() | dirty_for_render);

        // If node itself is dirty for render, we don't need to keep track of extra dirty rects
        self.needs_to_add_extra_dirty_rect_to_dirty_region.set(
            !dirty_for_render && (self.needs_to_add_extra_dirty_rect_to_dirty_region.get() || additional_dirty_region),
        );
    }

    pub fn recompute_own_properties(&self) {
        let mut set_dirty_bounds = self.content_changed.get() || self.delay_propagate_needs_bounds_update.get();
        let mut set_dirty_for_render = self.content_changed.get() || self.delay_propagate_is_dirty_for_render.get();
        let set_has_extra_dirty_rect = self.delay_propagate_has_extra_dirty_rects.get();

        // As upstream: the bounds-update flag is not part of this reset.
        self.delay_propagate_is_dirty_for_render.set(false);
        self.delay_propagate_has_extra_dirty_rects.set(false);

        self.enqueued_for_own_properties_recompute.set(false);
        if self.own_bounds_dirty.get() {
            self.own_content_bounds
                .set(self.compute_own_content_bounds().and_then(LtrbRect::null_if_zero_size));
            set_dirty_for_render = true;
            set_dirty_bounds = true;
        }

        if self.clip_size_dirty.get() {
            let mut clip: Option<LtrbRect> = None;
            if let Some(geometry) = self.clip() {
                clip = Some(LtrbRect::from_rect(geometry.bounds()));
            }
            if self.clip_to_bounds() {
                let size = self.size();
                let bounds = LtrbRect::new(0.0, 0.0, size.x, size.y);
                clip = Some(match clip {
                    Some(clip) => clip.intersect_or_empty(bounds),
                    None => bounds,
                });
            }
            // The WinUI clip object (not from upstream): the bounds of the
            // area it leaves visible.
            if let Some(shape) = self.composition_clip_shape() {
                let bounds = shape.bounds();
                clip = Some(match clip {
                    Some(clip) => clip.intersect_or_empty(bounds),
                    None => bounds,
                });
            }

            if self.own_clip_rect.get() != clip {
                self.own_clip_rect.set(clip);
                set_dirty_for_render = true;
                set_dirty_bounds = true;
            }
        }

        if self.combined_transform_dirty.get() {
            // With the WinUI members (the relative adjustments, the rotation
            // axis, pixel snapping) applied; a visual with a
            // `ParentForTransform` is completed by the adorner pass.
            self.own_transform.set(self.compute_own_transform_from_properties(self.transform_matrix()));
            if self.parent_for_transform().is_some() {
                self.adorner_helper_enqueue_for_adorner_update();
            }

            set_dirty_for_render = true;
            set_dirty_bounds = true;

            self.att_helper_combined_transform_changed();
        }

        set_dirty_for_render |= self.composition_fields_dirty.get();

        self.own_bounds_dirty.set(false);
        self.clip_size_dirty.set(false);
        self.combined_transform_dirty.set(false);
        self.composition_fields_dirty.set(false);
        self.propagate_flags(set_dirty_bounds, set_dirty_for_render, set_has_extra_dirty_rect);
    }
}
