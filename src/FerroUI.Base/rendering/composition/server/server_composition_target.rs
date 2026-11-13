use super::{
    impl_animated_server_object, CompositionTargetOverlays, DebugEventsDirtyRectCollectorProxy, IAnimatedServerObject,
    IDirtyRectCollector, IDirtyRectTracker, IServerObject, MultiDirtyRectTracker, RegionDirtyRectTracker,
    ServerCompositionVisual, ServerCompositor, ServerObject, SingleDirtyRectTracker,
};
use crate::media::Colors;
use crate::platform::surfaces::IPlatformRenderSurface;
use crate::platform::{
    IDrawingContextImpl, IDrawingContextLayerImpl, IRenderTarget, LtrbRect, RenderTargetError, RenderTargetSceneInfo,
};
use crate::rendering::composition::generated::{
    CompositionTargetChangedFields, ServerCompositionTargetHooks, ServerCompositionTargetProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::rendering::composition::{CompositionTransparencyLevel, ICompositionTargetDebugEvents};
use crate::rendering::{LayoutPassTiming, RendererDebugOverlays};
use crate::{Matrix, PixelRect, PixelSize, Size};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// Returns the current platform surfaces of a composition target.
///
/// The thread that renders calls it and uses what it returns: the function
/// and the surfaces are shared between the threads.
pub type RenderSurfaces = std::sync::Arc<dyn Fn() -> Vec<std::sync::Arc<dyn IPlatformRenderSurface>> + Send + Sync>;

/// Server-side counterpart of the `CompositionTarget`. That's the place
/// where we update visual transforms, track dirty rects and actually do
/// rendering.
pub struct ServerCompositionTarget {
    this: Weak<ServerCompositionTarget>,
    object: ServerObject,
    compositor: Weak<ServerCompositor>,
    props: ServerCompositionTargetProps,
    surfaces: RenderSurfaces,
    overlays: CompositionTargetOverlays,
    render_target: RefCell<Option<Rc<dyn IRenderTarget>>>,
    layer_size: Cell<PixelSize>,
    layer: RefCell<Option<Rc<dyn IDrawingContextLayerImpl>>>,
    update_requested: Cell<bool>,
    redraw_requested: Cell<bool>,
    full_redraw_requested: Cell<bool>,
    disposed: Cell<bool>,
    attached_visuals: RefCell<HashMap<*const ServerCompositionVisual, Rc<ServerCompositionVisual>>>,
    dirty_rects: Rc<dyn IDirtyRectTracker>,
    id: i64,
    debug_events: RefCell<Option<Arc<dyn ICompositionTargetDebugEvents>>>,
    rendered_visuals: Cell<i32>,
    visited_visuals: Cell<i32>,
    is_waiting_for_ready_render_target: Cell<bool>,
    is_waiting_for_render_loop_wakeup: Cell<bool>,
    /// The times of the frames the target drew, when the environment asks
    /// for them (`FERROUI_FRAME_TIMES=1`): a summary is printed to the
    /// standard error when the target is disposed. Not from the reference: a
    /// diagnostic that measures render backends in a window on screen
    /// (docs/porting/vello-backend.md, section 14).
    frame_times: RefCell<Option<Vec<std::time::Duration>>>,
    /// The time in milliseconds from which a frame is printed to the
    /// standard error with its number and time, when the environment asks
    /// for it (`FERROUI_SLOW_FRAME_MS`): what finds the frame that stalls
    /// (docs/porting/gpu-rendering.md, "Pipelines made ahead of their first
    /// draw"). Not from the reference.
    slow_frame_ms: Option<f64>,
    /// The counters of the render thread at the last report of them
    /// (`report_render_counters`).
    render_counters_reported: RefCell<Option<crate::diagnostics::perf_counters::PerfCountersSnapshot>>,
    /// How many frames the target drew while it measures them.
    frames_drawn: Cell<u64>,
    /// The generation of the effect prewarm list
    /// (`effects::register_effect_prewarm`) the target prewarmed up to.
    prewarmed_effects: Cell<u64>,
    /// Whether the environment turns the prewarm of the effects off
    /// (`FERROUI_EFFECT_PREWARM=0`): what measures a frame without it.
    effect_prewarm_disabled: bool,
    /// Whether the render that is running drew a frame.
    drew_frame: Cell<bool>,
    /// Whether the render that is running drew no scene: all it did was to
    /// present the frame its layer held back.
    only_presented_held_frame: Cell<bool>,
    /// Whether the layer was drawn and not blitted onto the target since: a
    /// tick drew it ahead of the target (section 9.51 of
    /// desktop-performance.md) and had nothing new to present, or the
    /// target was not ready. The next tick presents it.
    layer_unpresented: Cell<bool>,
    /// The backdrops drawn in the frames so far (winui-materials.md): the
    /// visual that painted each and its area in device pixels. A dirty
    /// rectangle that touches one makes all of it dirty.
    backdrop_regions: RefCell<Vec<(Weak<ServerCompositionVisual>, LtrbRect)>>,
}

/// The side, in logical units, of the offscreen scene the effects are
/// prewarmed in; each is drawn over the middle half of it.
const EFFECT_PREWARM_SIZE: f64 = 128.0;

crate::ferro_thread_local! {
    /// The generation of the effect prewarm list each compositor of this
    /// thread prewarmed up to, by scaling (the programs of a blur depend on
    /// its radius in pixels).
    static PREWARMED_EFFECTS: RefCell<Vec<(Weak<ServerCompositor>, u64, u64)>> = const { RefCell::new(Vec::new()) };
}

fn compositor_prewarmed_effects(compositor: &Rc<ServerCompositor>, scaling: f64) -> u64 {
    PREWARMED_EFFECTS.with(|prewarmed| {
        let mut prewarmed = prewarmed.borrow_mut();
        prewarmed.retain(|(owner, ..)| owner.strong_count() > 0);
        prewarmed
            .iter()
            .find(|(owner, bits, _)| *bits == scaling.to_bits() && std::ptr::eq(owner.as_ptr(), Rc::as_ptr(compositor)))
            .map_or(0, |(.., generation)| *generation)
    })
}

fn set_compositor_prewarmed_effects(compositor: &Rc<ServerCompositor>, scaling: f64, generation: u64) {
    PREWARMED_EFFECTS.with(|prewarmed| {
        let mut prewarmed = prewarmed.borrow_mut();
        match prewarmed
            .iter_mut()
            .find(|(owner, bits, _)| *bits == scaling.to_bits() && std::ptr::eq(owner.as_ptr(), Rc::as_ptr(compositor)))
        {
            Some(entry) => entry.2 = entry.2.max(generation),
            None => prewarmed.push((Rc::downgrade(compositor), scaling.to_bits(), generation)),
        }
    });
}

impl ServerCompositionTarget {
    /// Whether the target has been disposed. A disposed target stays in
    /// the table of the compositor until its id is released.
    pub(crate) fn is_disposed(&self) -> bool {
        self.disposed.get()
    }

    /// `id` is the identity of the target, allocated by the UI thread so
    /// that it can match readback data against it.
    pub fn new(compositor: &Rc<ServerCompositor>, surfaces: RenderSurfaces, id: i64) -> Rc<ServerCompositionTarget> {
        let mut dirty_rects: Option<Rc<dyn IDirtyRectTracker>> = None;
        // Not the service locator: a target is created by the thread that
        // applies the batch. The render interface is lent for the call.
        compositor.render_interface().with_platform_render_interface(|platform_render| {
            if platform_render.supports_regions() && compositor.options().region_dirty_rect_clipping() {
                let max_rects = compositor.options().max_dirty_rects.unwrap_or(8);
                dirty_rects = Some(if max_rects <= 0 {
                    Rc::new(RegionDirtyRectTracker::new(platform_render))
                } else {
                    Rc::new(MultiDirtyRectTracker::new(
                        platform_render,
                        max_rects,
                        // WPF uses 50K, but that merges stuff rather aggressively
                        compositor.options().dirty_rect_merge_eagerness.unwrap_or(1000.0),
                    ))
                });
            }
        });
        let dirty_rects = dirty_rects.unwrap_or_else(|| Rc::new(SingleDirtyRectTracker::new()));

        Rc::new_cyclic(|this: &Weak<ServerCompositionTarget>| {
            let owner: Weak<dyn IAnimatedServerObject> = this.clone();
            ServerCompositionTarget {
                this: this.clone(),
                object: ServerObject::new(compositor, owner),
                compositor: Rc::downgrade(compositor),
                props: ServerCompositionTargetProps::new(),
                surfaces,
                overlays: CompositionTargetOverlays::new(compositor),
                render_target: RefCell::new(None),
                layer_size: Cell::new(PixelSize::default()),
                layer: RefCell::new(None),
                update_requested: Cell::new(false),
                redraw_requested: Cell::new(false),
                full_redraw_requested: Cell::new(false),
                disposed: Cell::new(false),
                attached_visuals: RefCell::new(HashMap::new()),
                dirty_rects,
                id,
                debug_events: RefCell::new(None),
                rendered_visuals: Cell::new(0),
                visited_visuals: Cell::new(0),
                is_waiting_for_ready_render_target: Cell::new(false),
                is_waiting_for_render_loop_wakeup: Cell::new(false),
                frame_times: RefCell::new(
                    std::env::var("FERROUI_FRAME_TIMES").is_ok_and(|value| value == "1").then(Vec::new),
                ),
                render_counters_reported: RefCell::new(None),
                slow_frame_ms: std::env::var("FERROUI_SLOW_FRAME_MS")
                    .ok()
                    .and_then(|value| value.trim().parse::<f64>().ok())
                    .filter(|value| value.is_finite() && *value >= 0.0),
                frames_drawn: Cell::new(0),
                prewarmed_effects: Cell::new(0),
                effect_prewarm_disabled: std::env::var("FERROUI_EFFECT_PREWARM").is_ok_and(|value| value == "0"),
                drew_frame: Cell::new(false),
                only_presented_held_frame: Cell::new(false),
                layer_unpresented: Cell::new(false),
                backdrop_regions: RefCell::new(Vec::new()),
            }
        })
    }

    /// Makes dirty the whole of every backdrop a dirty rectangle touches,
    /// until no more is added: a blurred backdrop drawn in part would mix the
    /// new pixels with the old ones (winui-materials.md, the backdrop).
    fn expand_dirty_rects_over_backdrops(&self) {
        let mut regions = self.backdrop_regions.borrow_mut();
        regions.retain(|(visual, _)| visual.upgrade().is_some_and(|visual| visual.root().is_some()));
        if regions.is_empty() || self.dirty_rects.is_empty() {
            return;
        }
        let mut added = vec![false; regions.len()];
        loop {
            let mut changed = false;
            for (index, (_, region)) in regions.iter().enumerate() {
                if !added[index] && self.dirty_rects.intersects(*region) {
                    self.dirty_rects.add_rect(*region);
                    added[index] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }

    /// Keeps the backdrops drawn in the frame that just ended, replacing the
    /// areas of the visuals that drew again.
    fn record_backdrop_regions(&self) {
        let drawn = super::take_backdrop_regions();
        if drawn.is_empty() {
            return;
        }
        let mut regions = self.backdrop_regions.borrow_mut();
        for (visual, region) in drawn {
            regions.retain(|(existing, _)| !existing.ptr_eq(&visual));
            regions.push((visual, region));
        }
    }

    pub fn compositor(&self) -> Option<Rc<ServerCompositor>> {
        self.compositor.upgrade()
    }

    pub fn dirty_rects(&self) -> &Rc<dyn IDirtyRectTracker> {
        &self.dirty_rects
    }

    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn debug_events(&self) -> Option<Arc<dyn ICompositionTargetDebugEvents>> {
        self.debug_events.borrow().clone()
    }

    pub fn set_debug_events(&self, value: Option<Arc<dyn ICompositionTargetDebugEvents>>) {
        *self.debug_events.borrow_mut() = value;
    }

    pub fn rendered_visuals(&self) -> i32 {
        self.rendered_visuals.get()
    }

    pub fn visited_visuals(&self) -> i32 {
        self.visited_visuals.get()
    }

    pub fn pixel_size(&self) -> PixelSize {
        PixelSize::from_size_ceiling(self.size(), self.scaling())
    }

    /// Returns true if the target is enabled and has pending work but its
    /// render target was not ready.
    pub fn is_waiting_for_ready_render_target(&self) -> bool {
        self.is_waiting_for_ready_render_target.get()
    }

    /// Returns true if the target's render target is waiting for a render
    /// loop wakeup (i.e. the platform will call Wakeup() when ready, no
    /// need to keep polling).
    pub fn is_waiting_for_render_loop_wakeup(&self) -> bool {
        self.is_waiting_for_render_loop_wakeup.get()
    }

    // --- generated properties -------------------------------------------------

    pub fn root(&self) -> Option<Rc<ServerCompositionVisual>> {
        self.props.root().and_then(|o| o.into_any_rc().downcast::<ServerCompositionVisual>().ok())
    }

    pub fn is_enabled(&self) -> bool {
        self.props.is_enabled()
    }

    pub fn debug_overlays(&self) -> RendererDebugOverlays {
        self.props.debug_overlays()
    }

    pub fn last_layout_pass_timing(&self) -> LayoutPassTiming {
        self.props.last_layout_pass_timing()
    }

    pub fn scaling(&self) -> f64 {
        self.props.scaling()
    }

    pub fn size(&self) -> Size {
        self.props.size()
    }

    pub fn transparency_level(&self) -> CompositionTransparencyLevel {
        self.props.transparency_level()
    }

    pub fn platform_specific_scene_info(&self) -> Option<std::sync::Arc<dyn Any + Send + Sync>> {
        self.props.platform_specific_scene_info()
    }

    // ---------------------------------------------------------------------------

    pub fn update(self: &Rc<Self>, diagnostics_compositor_global_update_elapsed_time: Duration) {
        let Some(compositor) = self.compositor() else { return };
        if self.disposed.get() {
            compositor.remove_composition_target(self);
            return;
        }

        let Some(root) = self.root() else { return };

        self.overlays.record_global_compositor_update_time(self, diagnostics_compositor_global_update_elapsed_time);
        self.overlays.mark_update_call_start(self);

        let scaling = self.scaling();
        let transform = Matrix::create_scale(scaling, scaling);
        let tracker: Rc<dyn IDirtyRectCollector> = self.dirty_rects.clone();
        let collector: Rc<dyn IDirtyRectCollector> = match self.debug_events() {
            Some(events) => Rc::new(DebugEventsDirtyRectCollectorProxy::new(tracker, events)),
            None => tracker,
        };
        let pixel_size = self.pixel_size();
        root.update_root(
            collector,
            transform,
            LtrbRect::new(0.0, 0.0, pixel_size.width as f64, pixel_size.height as f64),
        );
        self.update_requested.set(false);
        self.overlays.mark_update_call_end(self);
    }

    pub fn render(self: &Rc<Self>) {
        self.drew_frame.set(false);
        self.only_presented_held_frame.set(false);
        if self.frame_times.borrow().is_none() && self.slow_frame_ms.is_none() {
            return self.render_frame();
        }
        let start = std::time::Instant::now();
        self.render_frame();
        if self.drew_frame.get() {
            let elapsed = start.elapsed();
            if let Some(times) = self.frame_times.borrow_mut().as_mut() {
                times.push(elapsed);
            }
            let frame = self.frames_drawn.get();
            self.frames_drawn.set(frame + 1);
            let ms = elapsed.as_secs_f64() * 1000.0;
            if self.slow_frame_ms.is_some_and(|threshold| ms >= threshold) {
                eprintln!("slow frame of composition target {}: frame {frame} took {ms:.3} ms", self.id);
            }
        }
    }

    /// Whether the last [`render`](Self::render) got as far as drawing a
    /// frame.
    pub(crate) fn drew_frame(&self) -> bool {
        self.drew_frame.get()
    }

    /// Whether the last render drew no scene: all it did was to present the
    /// frame the layer held back.
    pub(crate) fn only_presented_held_frame(&self) -> bool {
        self.only_presented_held_frame.get()
    }

    /// Whether the layer of the target holds a frame back: what the target
    /// drew last is presented by its next frame
    /// ([`IDrawingContextLayerImpl::has_pending_frame`]).
    pub(crate) fn has_pending_frame(&self) -> bool {
        self.layer_unpresented.get() || self.layer.borrow().as_ref().is_some_and(|layer| layer.has_pending_frame())
    }

    /// The layer of the target, when this frame may be drawn into it before
    /// the target is asked for its drawing context: the target and the layer
    /// allow it, the batches miss display frames
    /// ([`ServerCompositor::draws_layers_ahead`]), the layer is the one the
    /// frame needs, and nothing is drawn over it on the target (section 9.51
    /// of desktop-performance.md).
    fn layer_to_draw_ahead(
        &self,
        compositor: &ServerCompositor,
        render_target: &Rc<dyn IRenderTarget>,
        scene_info: &RenderTargetSceneInfo,
    ) -> Option<Rc<dyn IDrawingContextLayerImpl>> {
        if !compositor.draws_layers_ahead()
            || self.overlays.require_layer()
            || !render_target.prepare_frame_ahead(scene_info)
        {
            return None;
        }
        let layer = self.layer.borrow().clone()?;
        (scene_info.size == self.layer_size.get() && !layer.is_corrupted() && layer.draws_ahead_of_target())
            .then_some(layer)
    }

    /// The beginning of a frame that draws the tree: the dirty rectangles
    /// are final and the frame is counted.
    fn begin_scene(&self, render_bounds: LtrbRect, pixel_size: PixelSize) {
        use crate::diagnostics::perf_counters::{add, PerfCounter};
        super::take_backdrop_regions();
        self.dirty_rects.finalize_frame(render_bounds);
        let dirty = self.dirty_rects.combined_rect().intersect_or_empty(render_bounds);
        add(PerfCounter::RenderFramesDrawn, 1);
        add(PerfCounter::RenderDirtyBoundsPixels, (dirty.width().max(0.0) * dirty.height().max(0.0)) as u64);
        add(PerfCounter::RenderTargetPixels, (pixel_size.width as u64) * (pixel_size.height as u64));
    }

    /// The end of a frame: nothing is dirty any more.
    fn end_frame(&self, render_bounds: LtrbRect) {
        self.rendered_visuals.set(0);
        self.visited_visuals.set(0);
        self.redraw_requested.set(false);
        self.dirty_rects.initialize(render_bounds);
    }

    /// The times of the frames the target drew, as a line of text: their
    /// count, median, mean, 95th percentile and longest.
    fn frame_times_summary(&self) -> Option<String> {
        let mut times = self.frame_times.borrow().clone()?;
        if times.is_empty() {
            return None;
        }
        times.sort();
        let ms = |duration: std::time::Duration| duration.as_secs_f64() * 1000.0;
        let mean = times.iter().map(|time| ms(*time)).sum::<f64>() / times.len() as f64;
        Some(format!(
            "frame times of composition target {}: {} frames; ms per frame: median {:.3}, mean {:.3}, p95 {:.3}, max {:.3}",
            self.id,
            times.len(),
            ms(times[times.len() / 2]),
            mean,
            ms(times[(times.len() * 95 / 100).min(times.len() - 1)]),
            ms(times[times.len() - 1]),
        ))
    }

    fn render_frame(self: &Rc<Self>) {
        self.is_waiting_for_ready_render_target.set(false);
        self.is_waiting_for_render_loop_wakeup.set(false);

        if self.disposed.get() {
            return;
        }

        let Some(root) = self.root() else { return };
        let Some(compositor) = self.compositor() else { return };

        let corrupted = self.render_target.borrow().as_ref().is_some_and(|t| t.platform_render_target_state().is_corrupted);
        if corrupted {
            if let Some(layer) = self.layer.borrow_mut().take() {
                layer.dispose();
            }
            self.layer_unpresented.set(false);
            if let Some(render_target) = self.render_target.borrow_mut().take() {
                render_target.dispose();
            }
            self.redraw_requested.set(true);
        }

        if self.render_target.borrow().is_none() {
            let surfaces = (self.surfaces)();
            if !compositor.is_ready_to_create_render_target(&surfaces) {
                self.is_waiting_for_ready_render_target.set(self.is_enabled());
                return;
            }
            let render_target = compositor.create_render_target(&surfaces);
            *self.render_target.borrow_mut() = Some(render_target);
        }
        let Some(render_target) = self.render_target.borrow().clone() else { return };

        // A frame the layer holds back is presented by this tick, also when
        // the tick has nothing to draw (section 9.33 of
        // desktop-performance.md).
        let present_pending = self.has_pending_frame();
        if self.dirty_rects.is_empty()
            && !self.redraw_requested.get()
            && !self.update_requested.get()
            && !present_pending
        {
            return;
        }

        self.redraw_requested.set(self.redraw_requested.get() | !self.dirty_rects.is_empty());

        if !self.redraw_requested.get() && !present_pending {
            return;
        }

        let state = render_target.platform_render_target_state();
        if !state.is_ready {
            self.is_waiting_for_ready_render_target.set(self.is_enabled());
            self.is_waiting_for_render_loop_wakeup
                .set(self.is_enabled() && state.will_wake_up_render_loop_when_ready);
            return;
        }

        let properties = render_target.properties();
        let need_layer = self.overlays.require_layer() // Check if we don't need overlays
            // Check if render target can be rendered to directly and preserves the previous frame
            || !(properties.retains_previous_frame_contents && properties.is_suitable_for_direct_rendering);

        let pixel_size = self.pixel_size();
        let scene_info = RenderTargetSceneInfo {
            size: pixel_size,
            scaling: self.scaling(),
            logical_size: self.size(),
            transparency_level: self.transparency_level(),
            platform_specific_scene_info: self.platform_specific_scene_info(),
        };
        self.drew_frame.set(true);
        let render_bounds = LtrbRect::new(0.0, 0.0, pixel_size.width as f64, pixel_size.height as f64);

        // Where the target and its layer allow it, the tree is drawn into
        // the layer before the target is asked for its drawing context
        // (section 9.51 of desktop-performance.md): the wait for the surface
        // of the target comes after the drawing, not before it.
        let mut drawn_ahead = false;
        let layer_ahead = need_layer.then(|| self.layer_to_draw_ahead(&compositor, &render_target, &scene_info)).flatten();
        if let Some(layer) = layer_ahead {
            let held_before = self.has_pending_frame();
            if self.full_redraw_requested.replace(false) {
                self.dirty_rects.initialize(render_bounds);
                self.dirty_rects.add_rect(render_bounds);
            }
            self.expand_dirty_rects_over_backdrops();
            if !self.dirty_rects.is_empty() {
                self.begin_scene(render_bounds, pixel_size);
                let mut context = layer.create_drawing_context();
                self.render_root_to_context_with_clip(&compositor, &mut *context, &root);
                context.dispose();
                self.record_backdrop_regions();
                self.layer_unpresented.set(true);
                drawn_ahead = true;
                // The layer holds this frame back and held none before it:
                // what a blit shows is the frame the target presented last.
                // The tick presents nothing, and does not wait for a surface
                // of the target; the next tick presents this frame.
                if !held_before && layer.has_pending_frame() {
                    self.end_frame(render_bounds);
                    self.report_render_counters();
                    return;
                }
            }
        }

        let (mut render_target_context, properties) = match render_target.try_create_drawing_context(&scene_info) {
            Ok(created) => created,
            Err(error) => {
                // A frame drawn ahead stays in the layer, and is presented
                // by the next tick that has the target.
                if drawn_ahead {
                    self.end_frame(render_bounds);
                }
                if matches!(error, RenderTargetError::NotReady(_)) {
                    self.is_waiting_for_ready_render_target.set(self.is_enabled());
                }
                return;
            }
        };

        {
            let mut full_redraw = false;

            let layer_is_stale = {
                let layer = self.layer.borrow();
                pixel_size != self.layer_size.get() || layer.as_ref().is_none_or(|layer| layer.is_corrupted())
            };
            if need_layer && layer_is_stale {
                if let Some(layer) = self.layer.borrow_mut().take() {
                    layer.dispose();
                }
                *self.layer.borrow_mut() = Some(render_target_context.create_layer(pixel_size));
                self.layer_size.set(pixel_size);
                self.layer_unpresented.set(false);
                full_redraw = true;
            } else if !need_layer {
                if let Some(layer) = self.layer.borrow_mut().take() {
                    layer.dispose();
                }
                self.layer_unpresented.set(false);
            }

            if self.full_redraw_requested.get() || (!need_layer && !properties.previous_frame_is_retained) {
                self.full_redraw_requested.set(false);
                full_redraw = true;
            }

            if full_redraw {
                self.dirty_rects.initialize(render_bounds);
                self.dirty_rects.add_rect(render_bounds);
            }

            if !drawn_ahead {
                self.expand_dirty_rects_over_backdrops();
            }
            let layer = self.layer.borrow().clone();
            // Whether the tick draws the tree now. A tick that does not
            // still presents the frame it drew ahead, or the frame its layer
            // holds back.
            let draws = !drawn_ahead && !self.dirty_rects.is_empty();
            if draws || self.has_pending_frame() {
                if draws {
                    self.begin_scene(render_bounds, pixel_size);
                }
                match layer {
                    Some(layer) => {
                        if draws {
                            let mut context = layer.create_drawing_context();
                            self.render_root_to_context_with_clip(&compositor, &mut *context, &root);
                            context.dispose();
                        } else if !drawn_ahead {
                            layer.complete_pending_frame();
                            self.only_presented_held_frame.set(true);
                        }
                        self.layer_unpresented.set(false);

                        render_target_context.clear(Colors::TRANSPARENT);
                        render_target_context.set_transform(Matrix::IDENTITY);
                        if layer.can_blit() {
                            layer.blit(&mut *render_target_context);
                        } else {
                            let rect = PixelRect::from_size(pixel_size).to_rect(1.0);
                            render_target_context.draw_bitmap(&*layer, 1.0, rect, rect);
                        }
                        self.overlays.draw(self, &mut *render_target_context, true);
                    }
                    // Without a layer no frame is held back: the tick draws.
                    None => {
                        self.render_root_to_context_with_clip(&compositor, &mut *render_target_context, &root);
                        self.overlays.draw(self, &mut *render_target_context, false);
                    }
                }
                if draws {
                    self.record_backdrop_regions();
                }
            }

            self.end_frame(render_bounds);
        }
        render_target_context.dispose();
        self.report_render_counters();

        self.prewarm_effects(&compositor, &render_target, &scene_info);
    }

    /// With the feature `perf-counters` and `FERROUI_FRAME_TIMES=1`: every 300 frames drawn,
    /// prints what the render thread counted since the last report, a frame. The counters are
    /// those of this thread, so with several targets a report covers the frames of all of them.
    fn report_render_counters(&self) {
        use crate::diagnostics::perf_counters::{self, PerfCounter};
        if !perf_counters::ENABLED || self.frame_times.borrow().is_none() {
            return;
        }
        let now = perf_counters::snapshot();
        let mut last = self.render_counters_reported.borrow_mut();
        let counted = match last.as_ref() {
            Some(last) => now.since(last),
            None => now.clone(),
        };
        let frames = counted.get(PerfCounter::RenderFramesDrawn);
        if frames < 300 {
            return;
        }
        eprintln!("render thread counters of composition target {}:\n{}", self.id, counted.report_per(frames, "frame"));
        *last = Some(now);
    }

    /// After a frame: has the render target make the GPU programs of the
    /// effects registered for the prewarm (`effects::register_effect_prewarm`,
    /// #437) that neither this target nor another target of the compositor
    /// at the same scaling prewarmed, by drawing each once offscreen, through
    /// the painting of the effect brushes, at the scaling of the scene. A
    /// target tries each generation of the list once: one that does not
    /// prewarm (a raster target) is not asked again.
    fn prewarm_effects(
        &self,
        compositor: &Rc<ServerCompositor>,
        render_target: &Rc<dyn IRenderTarget>,
        scene_info: &RenderTargetSceneInfo,
    ) {
        use crate::rendering::composition::effects::{
            effect_prewarm_effects_since, effect_prewarm_generation, effect_prewarm_since,
        };

        let prewarmed = self.prewarmed_effects.get();
        if self.effect_prewarm_disabled || effect_prewarm_generation() <= prewarmed {
            return;
        }
        let scaling = scene_info.scaling;
        let floor = prewarmed.max(compositor_prewarmed_effects(compositor, scaling));
        let (generation, graphs) = effect_prewarm_since(floor);
        // The custom effects that are runtime shaders (#455, E4).
        let effects = effect_prewarm_effects_since(floor);
        self.prewarmed_effects.set(generation);
        if graphs.is_empty() && effects.is_empty() {
            return;
        }

        let started = std::time::Instant::now();
        let side = (EFFECT_PREWARM_SIZE * scaling).ceil().clamp(1.0, 4096.0) as i32;
        let prewarm_scene = RenderTargetSceneInfo {
            size: PixelSize::new(side, side),
            logical_size: Size::new(EFFECT_PREWARM_SIZE, EFFECT_PREWARM_SIZE),
            ..scene_info.clone()
        };
        let area = crate::Rect::new(
            EFFECT_PREWARM_SIZE / 4.0,
            EFFECT_PREWARM_SIZE / 4.0,
            EFFECT_PREWARM_SIZE / 2.0,
            EFFECT_PREWARM_SIZE / 2.0,
        );
        let mut stand_in: Option<Option<Arc<crate::platform::SharedBitmapImpl>>> = None;
        let drew = render_target.prewarm(&prewarm_scene, &mut |context: &mut dyn IDrawingContextImpl| {
            // The source parameters are drawn as the noise of the materials:
            // an image decoded as the images of the application are.
            let stand_in = stand_in
                .get_or_insert_with(|| {
                    compositor
                        .render_interface()
                        .with_platform_render_interface(|render_interface| {
                            render_interface.load_bitmap(&mut std::io::Cursor::new(crate::media::NOISE_ASSET_PNG)).ok()
                        })
                        .flatten()
                })
                .clone();
            context.set_transform(Matrix::create_scale(scaling, scaling));
            for graph in &graphs {
                crate::rendering::composition::brushes::paint_effect_graph_for_prewarm(
                    compositor,
                    graph.clone(),
                    stand_in.clone(),
                    context,
                    area,
                );
            }
            for effect in &effects {
                crate::rendering::composition::brushes::paint_custom_effect_for_prewarm(
                    effect,
                    stand_in.clone(),
                    context,
                    area,
                );
            }
        });
        if !drew {
            return;
        }
        set_compositor_prewarmed_effects(compositor, scaling, generation);
        if self.frame_times.borrow().is_some() || self.slow_frame_ms.is_some() {
            eprintln!(
                "effect prewarm of composition target {}: {} effects drawn offscreen in {:.3} ms",
                self.id,
                graphs.len() + effects.len(),
                started.elapsed().as_secs_f64() * 1000.0
            );
        }
    }

    fn render_root_to_context_with_clip(
        &self,
        compositor: &ServerCompositor,
        context: &mut dyn IDrawingContextImpl,
        root: &Rc<ServerCompositionVisual>,
    ) {
        let use_layer_clip = compositor.options().use_save_layer_root_clip.unwrap_or(false);
        self.dirty_rects.begin_draw(context);
        {
            context.clear(Colors::TRANSPARENT);
            if use_layer_clip {
                context.push_layer(self.dirty_rects.combined_rect().to_rect());
            }
            let scaling = self.scaling();
            context.set_transform(Matrix::create_scale(scaling, scaling));
            let pixel_size = self.pixel_size();
            let (visited, rendered) = root.render(
                context,
                LtrbRect::new(0.0, 0.0, pixel_size.width as f64, pixel_size.height as f64),
                Some(self.dirty_rects.clone()),
                true,
                false,
                false,
            );
            self.visited_visuals.set(visited);
            self.rendered_visuals.set(rendered);
            if let Some(events) = self.debug_events() {
                events.set_rendered_visuals(rendered);
                events.set_visited_visuals(visited);
            }

            if use_layer_clip {
                context.pop_layer();
            }
        }
        self.dirty_rects.end_draw(context);
    }

    pub fn request_update(&self) {
        self.update_requested.set(true);
    }

    pub fn reset_render_target(&self) {
        if self.layer.borrow().is_none() && self.render_target.borrow().is_none() {
            return;
        }
        let current = self.compositor().map(|compositor| compositor.render_interface().ensure_current());
        self.release_render_target();
        if let Some(current) = current {
            current.dispose();
        }
    }

    /// Releases the layer and the render target of the target, in the
    /// graphics context that is current: the caller has made the context of
    /// the compositor current, without asking for a backend context (see
    /// [`ServerCompositor::release_gpu_resources`]).
    pub(crate) fn release_render_target(&self) {
        if let Some(layer) = self.layer.borrow_mut().take() {
            layer.dispose();
        }
        if let Some(render_target) = self.render_target.borrow_mut().take() {
            render_target.dispose();
        }
    }

    pub fn add_visual(&self, visual: &Rc<ServerCompositionVisual>) {
        let added = self.attached_visuals.borrow_mut().insert(Rc::as_ptr(visual), visual.clone()).is_none();
        if added && self.is_enabled() {
            visual.activate();
        }
    }

    pub fn remove_visual(&self, visual: &Rc<ServerCompositionVisual>) {
        let removed = self.attached_visuals.borrow_mut().remove(&Rc::as_ptr(visual)).is_some();
        if removed && self.is_enabled() {
            visual.deactivate();
        }
    }

    pub fn request_full_redraw(&self) {
        self.redraw_requested.set(true);
    }
}

impl ServerCompositionTargetHooks for ServerCompositionTarget {
    fn deserialize_changes_extra(&self, _reader: &mut BatchStreamReader<'_>) {
        self.redraw_requested.set(true);
        self.full_redraw_requested.set(true);
    }

    fn on_fields_deserialized(&self, _changed: CompositionTargetChangedFields) {}

    fn on_is_enabled_changed(&self) {
        let (Some(compositor), Some(this)) = (self.compositor(), self.this.upgrade()) else { return };
        let visuals: Vec<_> = self.attached_visuals.borrow().values().cloned().collect();
        if self.is_enabled() {
            compositor.add_composition_target(&this);
            for visual in visuals {
                visual.activate();
            }
        } else {
            compositor.remove_composition_target(&this);
            for visual in visuals {
                visual.deactivate();
            }
        }
    }

    fn on_debug_overlays_changed(&self) {
        self.full_redraw_requested.set(true);
        self.overlays.on_changed(self.debug_overlays());
    }

    fn on_last_layout_pass_timing_changed(&self) {
        self.overlays.on_last_layout_pass_timing_changed(self, self.last_layout_pass_timing());
    }
}

impl_animated_server_object!(ServerCompositionTarget, object);

impl IServerObject for ServerCompositionTarget {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
    }

    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        if let Some(summary) = self.frame_times_summary() {
            eprintln!("{summary}");
        }
        self.reset_render_target();
        if let (Some(compositor), Some(this)) = (self.compositor(), self.this.upgrade()) {
            compositor.remove_composition_target(&this);
        }
        // The garbage collector reclaims the target and its tree upstream;
        // here the references that form cycles are dropped.
        self.attached_visuals.borrow_mut().clear();
        ServerCompositionTargetProps::id_of_root_property().set_field(self, None);
    }

    fn get_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}
