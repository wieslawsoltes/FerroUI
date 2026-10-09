use super::{
    impl_animated_server_object, CompositionTargetOverlays, DebugEventsDirtyRectCollectorProxy, IAnimatedServerObject,
    IDirtyRectCollector, IDirtyRectTracker, IServerObject, MultiDirtyRectTracker, RegionDirtyRectTracker,
    ServerCompositionVisual, ServerCompositor, ServerObject, SingleDirtyRectTracker,
};
use crate::media::Colors;
use crate::platform::surfaces::IPlatformRenderSurface;
use crate::platform::{
    IDrawingContextImpl, IDrawingContextLayerImpl, IRenderTarget, LtrbRect,
    RenderTargetSceneInfo,
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
            if platform_render.supports_regions() && compositor.options().use_region_dirty_rect_clipping == Some(true) {
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
            }
        })
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

        if self.dirty_rects.is_empty() && !self.redraw_requested.get() && !self.update_requested.get() {
            return;
        }

        self.redraw_requested.set(self.redraw_requested.get() | !self.dirty_rects.is_empty());

        if !self.redraw_requested.get() {
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
        let (mut render_target_context, properties) = render_target.create_drawing_context(&scene_info);

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
                full_redraw = true;
            } else if !need_layer {
                if let Some(layer) = self.layer.borrow_mut().take() {
                    layer.dispose();
                }
            }

            if self.full_redraw_requested.get() || (!need_layer && !properties.previous_frame_is_retained) {
                self.full_redraw_requested.set(false);
                full_redraw = true;
            }

            let render_bounds = LtrbRect::new(0.0, 0.0, pixel_size.width as f64, pixel_size.height as f64);

            if full_redraw {
                self.dirty_rects.initialize(render_bounds);
                self.dirty_rects.add_rect(render_bounds);
            }

            if !self.dirty_rects.is_empty() {
                self.dirty_rects.finalize_frame(render_bounds);
                let layer = self.layer.borrow().clone();
                match layer {
                    Some(layer) => {
                        {
                            let mut context = layer.create_drawing_context();
                            self.render_root_to_context_with_clip(&compositor, &mut *context, &root);
                            context.dispose();
                        }

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
                    None => {
                        self.render_root_to_context_with_clip(&compositor, &mut *render_target_context, &root);
                        self.overlays.draw(self, &mut *render_target_context, false);
                    }
                }
            }

            self.rendered_visuals.set(0);
            self.visited_visuals.set(0);

            self.redraw_requested.set(false);
            self.dirty_rects.initialize(render_bounds);
        }
        render_target_context.dispose();
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
