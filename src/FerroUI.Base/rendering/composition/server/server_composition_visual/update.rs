use super::walker::{walk, IServerTreeVisitor, TreeWalkContext};
use super::ServerCompositionVisual;
use crate::media::EffectExtensions;
use crate::platform::LtrbRect;
use crate::rendering::composition::server::{CompositorPools, IDirtyRectCollector};
use crate::Matrix;
use std::rc::Rc;

struct UpdateContext {
    context: TreeWalkContext,
    dirty_region: Rc<dyn IDirtyRectCollector>,
    dirty_region_disable_count: i32,
    dirty_region_disable_count_stack: Vec<i32>,
    dirty_region_collector_stack: Vec<Rc<dyn IDirtyRectCollector>>,
}

impl UpdateContext {
    fn new(pools: &CompositorPools, dirty_rects: Rc<dyn IDirtyRectCollector>, transform: Matrix, clip: LtrbRect) -> Self {
        Self {
            dirty_region: dirty_rects,
            context: TreeWalkContext::new(pools, transform, clip),
            dirty_region_disable_count: 0,
            dirty_region_disable_count_stack: pools.int_stack_pool.rent(),
            dirty_region_collector_stack: pools.dirty_rect_collector_stack_pool.rent(),
        }
    }

    fn are_dirty_regions_disabled(&self) -> bool {
        self.dirty_region_disable_count != 0
    }

    fn push_cache_if_needed(&mut self, visual: &ServerCompositionVisual) {
        if let Some(cache) = visual.cache() {
            let collector: Rc<dyn IDirtyRectCollector> = cache;
            self.dirty_region_collector_stack.push(std::mem::replace(&mut self.dirty_region, collector));
            self.dirty_region_disable_count_stack.push(self.dirty_region_disable_count);
            self.dirty_region_disable_count = 0;
            self.context.push_set_transform(Matrix::IDENTITY);
            self.context.reset_clip(LtrbRect::INFINITE);
        }
    }

    fn pop_cache_if_needed(&mut self, visual: &ServerCompositionVisual) {
        if let Some(cache) = visual.cache() {
            self.context.pop_clip();
            self.context.pop_transform();
            self.dirty_region = self.dirty_region_collector_stack.pop().expect("a collector was pushed");
            self.dirty_region_disable_count = self.dirty_region_disable_count_stack.pop().expect("a count was pushed");
            if cache.is_dirty() {
                self.add_to_dirty_region(visual.sub_tree_bounds.get());
            }
        }
    }

    fn need_to_push_bounds_affecting_properties(node: &ServerCompositionVisual) -> bool {
        node.is_dirty_for_render_in_subgraph.get()
            || node.needs_to_add_extra_dirty_rect_to_dirty_region.get()
            || node.content_changed.get()
    }

    fn finalize_subtree_bounds(node: &ServerCompositionVisual) {
        // WPF simply removes drawing commands from every visual in invisible subtree (on UI thread).
        // We set the bounds to null when computing subtree bounds for invisible nodes.
        if !node.visible() {
            node.sub_tree_bounds.set(None);
        }

        if let Some(mut bounds) = node.sub_tree_bounds.get() {
            if let Some(effect) = node.effect() {
                bounds = bounds.inflate(EffectExtensions::get_effect_output_padding(Some(&*effect)));
                node.sub_tree_bounds.set(Some(bounds));
            }

            if let Some(clip) = node.own_clip_rect.get() {
                node.sub_tree_bounds.set(bounds.intersect_or_null(clip));
            }
        }

        match (node.sub_tree_bounds.get(), node.own_transform.get()) {
            (None, _) => node.transformed_sub_tree_bounds.set(None),
            (Some(bounds), Some(transform)) => {
                node.transformed_sub_tree_bounds.set(Some(bounds.transform_to_aabb(transform)))
            }
            (Some(bounds), None) => node.transformed_sub_tree_bounds.set(Some(bounds)),
        }

        node.enqueue_for_readback_update();
    }

    fn add_to_dirty_region(&self, bounds: Option<LtrbRect>) {
        let Some(bounds) = bounds else { return };
        if self.dirty_region_disable_count != 0 {
            return;
        }

        let transformed = bounds.transform_to_aabb(self.context.transform).intersect_or_empty(self.context.clip);
        if transformed.is_zero_size() {
            return;
        }

        self.dirty_region.add_rect(transformed);
    }

    fn push_bounds_affecting_properties(&mut self, node: &ServerCompositionVisual) {
        if let Some(transform) = node.own_transform.get() {
            self.context.push_transform(transform);
        }

        if let Some(clip) = node.own_clip_rect.get() {
            self.context.push_clip(clip.transform_to_aabb(self.context.transform));
        }
    }

    fn pop_bounds_affecting_properties(&mut self, node: &ServerCompositionVisual) {
        if node.own_transform.get().is_some() {
            self.context.pop_transform();
        }
        if node.own_clip_rect.get().is_some() {
            self.context.pop_clip();
        }
    }

    fn dispose(self, pools: &CompositorPools) {
        pools.int_stack_pool.return_stack(self.dirty_region_disable_count_stack);
        pools.dirty_rect_collector_stack_pool.return_stack(self.dirty_region_collector_stack);
        self.context.dispose(pools);
    }
}

impl IServerTreeVisitor for UpdateContext {
    fn pre_subgraph(&mut self, node: &Rc<ServerCompositionVisual>) -> bool {
        let visit_children = node.is_dirty_for_render_in_subgraph.get() || node.needs_bounding_box_update.get();

        // If this node has an alpha mask an we caused its inner bounds to change
        // then treat the node as if _isDirtyForRender was set.
        if node.needs_bounding_box_update.get() && node.opacity_mask_brush().is_some() {
            node.is_dirty_for_render.set(true);
        }

        // Special handling for effects: just add the entire node's old subtree bounds as a dirty region
        // WPF does this because they had legacy effects with non-affine transforms, we do this because
        // it's something to be done in the future (maybe)
        if node.is_dirty_for_render.get()
            || (node.is_dirty_for_render_in_subgraph.get() && node.content.has_effect(node))
        {
            // If bounds haven't actually changed, there is no point in adding them now since they will be added
            // again in PostSubgraph.
            if node.needs_bounding_box_update.get() && !self.are_dirty_regions_disabled() {
                // We add this node's bbox to the dirty region. Alternatively we could walk the sub-graph and add the
                // bbox of each node's content to the dirty region. Note that this is much harder to do because if the
                // transform changes we don't know anymore the old transform. We would have to use to a two phased dirty
                // region algorithm.
                self.add_to_dirty_region(node.transformed_sub_tree_bounds.get());
            }

            // If we added a node in the parent chain to the bbox we don't need to add anything below this node
            // to the dirty region.
            self.dirty_region_disable_count += 1;
        }

        // If a node in the sub-graph of this node is dirty for render and we haven't collected the bbox of one of pNode's
        // ascendants as dirty region, then we need to maintain the transform and clip stack so that we have a world transform
        // when we need to collect the bbox of the descendant node that is dirty for render.  If something has changed
        // in the contents or subgraph, we need to update the cache on this node.
        if Self::need_to_push_bounds_affecting_properties(node) {
            // Dirty regions will be enabled if we haven't collected an ancestor's bbox or if they were re-enabled
            // by an ancestor's cache.
            if !self.are_dirty_regions_disabled() {
                self.push_bounds_affecting_properties(node);
            }

            self.push_cache_if_needed(node);
        }

        if node.needs_bounding_box_update.get() {
            // This node's bbox needs to be updated. We start out by setting his bbox to the bbox of its content. All its
            // children will union their bbox into their parent's bbox. PostSubgraph will clip the bbox and transform it
            // to outer space.
            node.sub_tree_bounds.set(node.own_content_bounds.get());
        }

        visit_children
    }

    fn post_subgraph(&mut self, node: &Rc<ServerCompositionVisual>) {
        let parent = node.parent();

        if node.needs_bounding_box_update.get() {
            //
            // If pNode's bbox got recomputed it is at this point still in inner
            // space. We need to apply the clip and transform.
            //
            Self::finalize_subtree_bounds(node);
        }

        //
        // Update state on the parent node if we have a parent.
        if let Some(parent) = parent {
            // Update the bounding box on the parent.
            if parent.needs_bounding_box_update.get() {
                parent
                    .sub_tree_bounds
                    .set(LtrbRect::full_union(parent.sub_tree_bounds.get(), node.transformed_sub_tree_bounds.get()));
            }
        }

        //
        // If there are additional dirty regions, pick them up. (Additional dirty regions are
        // specified before the tranform, i.e. in inner space, hence we have to pick them
        // up before we pop the transform from the transform stack.
        //
        if node.needs_to_add_extra_dirty_rect_to_dirty_region.get() {
            self.add_to_dirty_region(Some(node.extra_dirty_rect.get()));
        }

        // If we pushed transforms here, we need to pop them again.  If we're handling a cache we need
        // to finish handling it here as well.
        if Self::need_to_push_bounds_affecting_properties(node) {
            self.pop_cache_if_needed(node);

            if !self.are_dirty_regions_disabled() {
                self.pop_bounds_affecting_properties(node);
            }
        }

        // Special handling for effects: just add the entire node's old subtree bounds as a dirty region
        // WPF does this because they had legacy effects with non-affine transforms, we do this because
        // it's something to be done in the future (maybe)
        if node.is_dirty_for_render.get() || (node.is_dirty_for_render_in_subgraph.get() && node.effect().is_some()) {
            self.dirty_region_disable_count -= 1;
            self.add_to_dirty_region(node.transformed_sub_tree_bounds.get());
        }

        node.is_dirty_for_render.set(false);
        node.is_dirty_for_render_in_subgraph.set(false);
        node.needs_bounding_box_update.set(false);
        node.needs_to_add_extra_dirty_rect_to_dirty_region.set(false);
        node.content_changed.set(false);
    }
}

impl ServerCompositionVisual {
    pub fn update_root(self: &Rc<Self>, tracker: Rc<dyn IDirtyRectCollector>, transform: Matrix, clip: LtrbRect) {
        let Some(compositor) = self.compositor() else { return };
        let pools = compositor.pools();
        let mut context = UpdateContext::new(pools, tracker, transform, clip);
        walk(&mut context, self, pools);
        context.dispose(pools);
    }
}
