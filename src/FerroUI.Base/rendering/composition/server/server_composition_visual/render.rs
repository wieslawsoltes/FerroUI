use super::walker::{walk, IServerTreeVisitor, TreeWalkContext};
use super::ServerCompositionVisual;
use crate::media::{RenderOptions, TextOptions};
use crate::platform::{IDrawingContextImpl, LtrbRect};
use crate::rendering::composition::server::{CompositorPools, IDirtyRectTracker, ServerVisualRenderContext};
use crate::Matrix;
use std::rc::Rc;

pub(super) struct RenderContext<'a> {
    pub(super) canvas: &'a mut dyn IDrawingContextImpl,
    dirty_rects: Option<Rc<dyn IDirtyRectTracker>>,
    pub(super) pools: &'a CompositorPools,
    render_children: bool,
    walk_context: TreeWalkContext,
    opacity_stack: Vec<f64>,
    opacity: f64,
    full_skip: bool,
    used_cache: bool,
    rendered_visuals: i32,
    visited_visuals: i32,
    pub(super) root_visual: Rc<ServerCompositionVisual>,
    skip_next_visual_transform: bool,
    rendering_to_bitmap_cache: bool,
    pub(super) adorner_pushed_clip_stack: Option<Vec<i32>>,
    pub(super) current_adorner_layer: Option<Rc<ServerCompositionVisual>>,
}

impl<'a> RenderContext<'a> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        root_visual: Rc<ServerCompositionVisual>,
        canvas: &'a mut dyn IDrawingContextImpl,
        mut dirty_rects: Option<Rc<dyn IDirtyRectTracker>>,
        pools: &'a CompositorPools,
        matrix: Matrix,
        mut clip: LtrbRect,
        render_children: bool,
        skip_root_visual_transform: bool,
        rendering_to_bitmap_cache: bool,
    ) -> Self {
        if let Some(tracker) = &dirty_rects {
            let dirty_clip = tracker.combined_rect();
            let is_single = tracker.is_single_dirty_rect_tracker();
            if is_single {
                dirty_rects = None;
            }
            clip = clip.intersect_or_empty(dirty_clip);
        }

        Self {
            canvas,
            dirty_rects,
            pools,
            render_children,
            root_visual,
            walk_context: TreeWalkContext::new(pools, matrix, clip),
            opacity: 1.0,
            opacity_stack: pools.double_stack_pool.rent(),
            full_skip: false,
            used_cache: false,
            rendered_visuals: 0,
            visited_visuals: 0,
            skip_next_visual_transform: skip_root_visual_transform,
            rendering_to_bitmap_cache,
            adorner_pushed_clip_stack: None,
            current_adorner_layer: None,
        }
    }

    fn handle_pre_graph_transform_clip_opacity(&mut self, visual: &ServerCompositionVisual) -> bool {
        let Some(transformed_sub_tree_bounds) = visual.transformed_sub_tree_bounds.get() else { return false };
        if !visual.visible() {
            return false;
        }
        let visual_opacity = visual.opacity() as f64;
        let effective_opacity = visual_opacity * self.opacity;
        if effective_opacity <= 0.003 {
            return false;
        }

        let mut effective_new_transform = self.walk_context.transform;
        if let Some(own_transform) = visual.own_transform.get() {
            if !self.skip_next_visual_transform {
                effective_new_transform = own_transform * self.walk_context.transform;
            }
        }
        self.skip_next_visual_transform = false;

        let mut effective_clip = self.walk_context.clip;
        if let Some(own_clip) = visual.own_clip_rect.get() {
            effective_clip = effective_clip.intersect_or_empty(own_clip.transform_to_aabb(effective_new_transform));
        }

        let world_bounds = transformed_sub_tree_bounds.transform_to_aabb(self.walk_context.transform);
        if !effective_clip.intersects(world_bounds)
            || self.dirty_rects.as_ref().is_some_and(|d| !d.intersects(world_bounds))
        {
            return false;
        }

        self.rendered_visuals += 1;

        // We are still in parent's coordinate space here

        if visual_opacity != 1.0 {
            self.opacity_stack.push(self.opacity);
            self.opacity = effective_opacity;
            self.canvas.push_opacity(visual_opacity, Some(transformed_sub_tree_bounds.to_rect()));
        }

        // Switch coordinate space to this visual's space

        if visual.own_transform.get().is_some() {
            self.walk_context.push_set_transform(effective_new_transform); // Reuse one computed before
            self.canvas.set_transform(effective_new_transform);
        }

        if visual.own_clip_rect.get().is_some() {
            self.walk_context.push_clip(effective_clip);
        }

        if visual.clip_to_bounds() {
            visual.content.push_clip_to_bounds(visual, self.canvas);
        }

        if let Some(clip) = visual.clip() {
            self.canvas.push_geometry_clip(&*clip);
        }

        true
    }

    fn dispose(mut self) {
        let pools = self.pools;
        self.adorner_helper_dispose();
        pools.double_stack_pool.return_stack(self.opacity_stack);
        self.walk_context.dispose(pools);
    }
}

impl IServerTreeVisitor for RenderContext<'_> {
    fn pre_subgraph(&mut self, visual: &Rc<ServerCompositionVisual>) -> bool {
        self.visited_visuals += 1;
        let bitmap_cache_root = self.rendering_to_bitmap_cache && Rc::ptr_eq(visual, &self.root_visual);
        if !bitmap_cache_root {
            // Skip those for the root visual if we are rendering to bitmap cache
            // Push transform, clip, opacity and check if those make the visual effectively invisible
            if !self.handle_pre_graph_transform_clip_opacity(visual) {
                self.full_skip = true;
                return false;
            }

            // Push adorner clip
            if visual.adorned_visual().is_some() {
                self.adorner_helper_render_pre_graph_push_adorner_clip(visual);
            }

            // If caching is enabled, draw from cache and skip rendering
            if let Some(cache) = visual.cache() {
                let (visited, rendered) = cache.draw(self.canvas);
                self.visited_visuals += visited;
                self.rendered_visuals += rendered;
                self.used_cache = true;
                return false;
            }
        }

        if visual.render_options() != RenderOptions::default() {
            self.canvas.push_render_options(visual.render_options());
        }

        if visual.text_options() != TextOptions::default() {
            self.canvas.push_text_options(visual.text_options());
        }

        if let (Some(mask), Some(bounds)) = (visual.opacity_mask_brush(), visual.sub_tree_bounds.get()) {
            self.canvas.push_opacity_mask(&*mask, bounds.to_rect());
        }

        if let (Some(effect), Some(bounds)) = (visual.effect(), visual.sub_tree_bounds.get()) {
            if let Some(effects) = self.canvas.as_drawing_context_impl_with_effects() {
                effects.push_effect(Some(bounds.to_rect()), &*effect);
            }
        }

        let clip = self.walk_context.clip;
        visual.render_core(&mut ServerVisualRenderContext::new(self.canvas), clip);

        self.render_children
    }

    fn post_subgraph(&mut self, visual: &Rc<ServerCompositionVisual>) {
        if self.full_skip {
            self.full_skip = false;
            return;
        }

        let bitmap_cache_root = self.rendering_to_bitmap_cache && Rc::ptr_eq(visual, &self.root_visual);

        // If we've used cache, those never got pushed in PreSubgraph
        if !self.used_cache {
            if visual.effect().is_some() && visual.sub_tree_bounds.get().is_some() {
                if let Some(effects) = self.canvas.as_drawing_context_impl_with_effects() {
                    effects.pop_effect();
                }
            }

            if visual.opacity_mask_brush().is_some() && visual.sub_tree_bounds.get().is_some() {
                self.canvas.pop_opacity_mask();
            }

            if visual.text_options() != TextOptions::default() {
                self.canvas.pop_text_options();
            }

            if visual.render_options() != RenderOptions::default() {
                self.canvas.pop_render_options();
            }
        }

        // A cached visual is always a leaf in the walk (children are skipped), so its
        // PreSubgraph/PostSubgraph are adjacent. Reset here so the flag never leaks into a
        // later sibling and causes its effect/mask/options pushes to be left unpopped.
        self.used_cache = false;

        // If we are rendering to bitmap cache, PreSubgraph skipped those for the root visual
        if !bitmap_cache_root {
            if visual.adorned_visual().is_some() {
                self.adorner_helper_render_post_graph_push_adorner_clip(visual);
            }

            if visual.clip().is_some() {
                self.canvas.pop_geometry_clip();
            }

            if visual.clip_to_bounds() {
                self.canvas.pop_clip();
            }

            if visual.own_clip_rect.get().is_some() {
                self.walk_context.pop_clip();
            }

            if visual.own_transform.get().is_some() {
                self.walk_context.pop_transform();
                self.canvas.set_transform(self.walk_context.transform);
            }

            if visual.opacity() != 1.0 {
                self.canvas.pop_opacity();
                self.opacity = self.opacity_stack.pop().expect("an opacity was pushed");
            }
        }
    }
}

impl ServerCompositionVisual {
    /// Renders the visual and, with `render_children`, its subtree onto
    /// `canvas`. Returns the numbers of visited and rendered visuals.
    pub fn render(
        self: &Rc<Self>,
        canvas: &mut dyn IDrawingContextImpl,
        clip: LtrbRect,
        dirty_rects: Option<Rc<dyn IDirtyRectTracker>>,
        render_children: bool,
        skip_root_visual_transform: bool,
        rendering_to_bitmap_cache: bool,
    ) -> (i32, i32) {
        let Some(compositor) = self.compositor() else { return (0, 0) };
        let pools = compositor.pools();
        let transform = canvas.transform();
        let mut render_context = RenderContext::new(
            self.clone(),
            canvas,
            dirty_rects,
            pools,
            transform,
            clip,
            render_children,
            skip_root_visual_transform,
            rendering_to_bitmap_cache,
        );
        walk(&mut render_context, self, pools);
        let result = (render_context.visited_visuals, render_context.rendered_visuals);
        render_context.dispose();
        result
    }
}
