use super::drawing::RenderDataDrawingContext;
use super::hit_testing::{GeometryCompositionHitTester, ICompositionHitTester, PointCompositionHitTester};
use super::server::RenderSurfaces;
use super::{
    CompositionDrawListVisual, CompositionTarget, CompositionTransparencyLevel, CompositionVisual, Compositor,
};
use crate::media::{DrawingContext, Geometry, GeometryHitTestResult, IntersectionResult, MediaContext};
use crate::reactive::{Disposable, IDisposable};
use crate::rendering::{
    IHitTester, IPresentationSource, IRenderer, RendererDiagnostics, SceneInvalidatedEventArgs,
};
use crate::threading::DispatcherPriority;
use crate::utilities::HandlerList;
use crate::{Point, Rect, Ref, Size, Visual};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_RENDERER_KEY: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// The renderers of this thread by key: lets the `Send` continuation of
    /// a batch find its renderer once it is back on the thread.
    static RENDERERS: RefCell<HashMap<u64, Weak<CompositingRenderer>>> = RefCell::new(HashMap::new());
}

/// A set of visuals that keeps the order in which they were added.
#[derive(Default)]
struct VisualSet {
    items: Vec<Ref<Visual>>,
    keys: HashSet<*const Visual>,
}

impl VisualSet {
    fn add(&mut self, visual: &Visual) {
        if self.keys.insert(visual as *const Visual) {
            self.items.push(visual.to_ref());
        }
    }

    fn contains(&self, visual: &Visual) -> bool {
        self.keys.contains(&(visual as *const Visual))
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn clear(&mut self) {
        self.items.clear();
        self.keys.clear();
    }
}

/// A renderer that utilizes a [`Compositor`] to render the visual tree.
pub struct CompositingRenderer {
    this: Weak<CompositingRenderer>,
    key: u64,
    root: Weak<dyn IPresentationSource>,
    compositor: Rc<Compositor>,
    recorder: RefCell<RenderDataDrawingContext>,
    dirty: RefCell<VisualSet>,
    recalculate_children: RefCell<VisualSet>,
    queued_update: Cell<bool>,
    queued_scene_invalidation: Cell<bool>,
    updating: Cell<bool>,
    is_disposed: Cell<bool>,
    composition_target: Rc<CompositionTarget>,
    diagnostics: Rc<RendererDiagnostics>,
    diagnostics_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    scene_invalidated: HandlerList<dyn Fn(&SceneInvalidatedEventArgs)>,
}

impl CompositingRenderer {
    /// Creates the renderer of the tree hosted by `root`. `surfaces`
    /// returns the platform surfaces to render to.
    ///
    /// The renderer refers to its presentation source weakly: the source
    /// owns its renderer.
    pub fn new(
        root: &Rc<dyn IPresentationSource>,
        compositor: &Rc<Compositor>,
        surfaces: RenderSurfaces,
    ) -> Rc<CompositingRenderer> {
        Self::with_weak_root(Rc::downgrade(root), compositor, surfaces)
    }

    /// Like [`new`](Self::new), for a presentation source that is still
    /// being constructed.
    pub fn with_weak_root(
        root: Weak<dyn IPresentationSource>,
        compositor: &Rc<Compositor>,
        surfaces: RenderSurfaces,
    ) -> Rc<CompositingRenderer> {
        let key = NEXT_RENDERER_KEY.fetch_add(1, Ordering::SeqCst);
        let renderer = Rc::new_cyclic(|this: &Weak<CompositingRenderer>| CompositingRenderer {
            this: this.clone(),
            key,
            root,
            compositor: compositor.clone(),
            recorder: RefCell::new(RenderDataDrawingContext::new(Some(compositor.clone()))),
            dirty: RefCell::new(VisualSet::default()),
            recalculate_children: RefCell::new(VisualSet::default()),
            queued_update: Cell::new(false),
            queued_scene_invalidation: Cell::new(false),
            updating: Cell::new(false),
            is_disposed: Cell::new(false),
            composition_target: compositor.create_composition_target(surfaces),
            diagnostics: RendererDiagnostics::new(),
            diagnostics_subscription: RefCell::new(None),
            scene_invalidated: HandlerList::new(),
        });
        let weak = Rc::downgrade(&renderer);
        let subscription = renderer.diagnostics.property_changed(move |diagnostics, property_name| {
            if let Some(renderer) = weak.upgrade() {
                renderer.on_diagnostics_property_changed(diagnostics, property_name);
            }
        });
        *renderer.diagnostics_subscription.borrow_mut() = Some(subscription);
        RENDERERS.with(|renderers| {
            let mut renderers = renderers.borrow_mut();
            renderers.retain(|_, r| r.strong_count() > 0);
            renderers.insert(key, Rc::downgrade(&renderer));
        });
        renderer
    }

    /// The composition target the renderer draws the tree to.
    pub fn composition_target(&self) -> &Rc<CompositionTarget> {
        &self.composition_target
    }

    /// The compositor of the renderer.
    pub fn compositor_handle(&self) -> &Rc<Compositor> {
        &self.compositor
    }

    /// Sets the visual whose composition visual is the root of the
    /// composition target; `None` detaches the tree.
    pub fn set_root(&self, root: Option<Ref<Visual>>) {
        let visual = root.and_then(|root| root.composition_visual()).map(|visual| (*visual).clone());
        self.composition_target.set_root(visual);
    }

    /// Sets the transparency level the composition target renders with.
    pub fn set_transparency_level(&self, level: CompositionTransparencyLevel) {
        self.composition_target.set_transparency_level(level);
    }

    /// Sets the platform-specific scene info passed to the render target.
    pub fn set_platform_specific_scene_info(&self, info: Option<std::sync::Arc<dyn Any + Send + Sync>>) {
        self.composition_target.set_platform_specific_scene_info(info);
    }

    /// The renderer as the hit tester of its scene.
    pub fn hit_tester(&self) -> Option<Rc<dyn IHitTester>> {
        self.this.upgrade().map(|this| this as Rc<dyn IHitTester>)
    }

    pub fn as_any(&self) -> &dyn Any {
        self
    }

    fn on_diagnostics_property_changed(&self, diagnostics: &RendererDiagnostics, property_name: &str) {
        if property_name == RendererDiagnostics::DEBUG_OVERLAYS_PROPERTY_NAME {
            self.composition_target.set_debug_overlays(diagnostics.debug_overlays());
        } else if property_name == RendererDiagnostics::LAST_LAYOUT_PASS_TIMING_PROPERTY_NAME {
            self.composition_target.set_last_layout_pass_timing(diagnostics.last_layout_pass_timing());
        }
    }

    fn queue_update(&self) {
        if self.queued_update.get() {
            return;
        }
        self.queued_update.set(true);
        let weak = self.this.clone();
        self.compositor.request_composition_update(move || {
            if let Some(renderer) = weak.upgrade() {
                renderer.update();
            }
        });
    }

    fn composition_filter<'a>(
        filter: Option<&'a dyn Fn(&Visual) -> bool>,
    ) -> Option<impl Fn(&Rc<CompositionVisual>) -> bool + 'a> {
        filter.map(|filter| {
            move |v: &Rc<CompositionVisual>| match CompositionDrawListVisual::from_visual(v) {
                Some(dlv) => dlv.visual().is_some_and(|visual| filter(&visual)),
                None => true,
            }
        })
    }

    fn hit_test_all<H: ICompositionHitTester>(
        &self,
        input: &H::Input,
        root: &Visual,
        filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<(Ref<Visual>, IntersectionResult)> {
        let Some(root_visual) = root.composition_visual() else { return Vec::new() };
        let root_visual: Rc<CompositionVisual> = (*root_visual).clone();

        let f = Self::composition_filter(filter);
        let f = f.as_ref().map(|f| f as &dyn Fn(&Rc<CompositionVisual>) -> bool);

        let Some(res) = self.composition_target.try_hit_test::<H>(input, Some(&root_visual), f) else {
            return Vec::new();
        };

        let mut result = Vec::new();
        for (intersection, v) in res {
            if let Some(visual) = CompositionDrawListVisual::from_visual(&v).and_then(|dv| dv.visual()) {
                if filter.is_none_or(|filter| filter(&visual)) {
                    result.push((visual, intersection));
                }
            }
        }
        result
    }

    fn hit_test_first_core<H: ICompositionHitTester>(
        &self,
        input: &H::Input,
        root: &Visual,
        filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<(Ref<Visual>, IntersectionResult)> {
        let root_visual = root.composition_visual()?;
        let root_visual: Rc<CompositionVisual> = (*root_visual).clone();

        let f = Self::composition_filter(filter);
        let f = f.as_ref().map(|f| f as &dyn Fn(&Rc<CompositionVisual>) -> bool);
        let result_filter = |v: &Rc<CompositionVisual>| CompositionDrawListVisual::from_visual(v).is_some();

        let (hit, intersection) =
            self.composition_target.try_hit_test_first::<H>(input, Some(&root_visual), f, Some(&result_filter));
        let visual = CompositionDrawListVisual::from_visual(&hit?)?.visual()?;
        Some((visual, intersection))
    }

    fn update_core(&self) {
        self.queued_update.set(false);
        let dirty: Vec<Ref<Visual>> = self.dirty.borrow().items.clone();
        for visual in &dirty {
            let Some(comp) = visual.composition_visual() else { continue };

            visual.synchronize_composition_properties();

            let draw_list = {
                struct ResetRecorder<'a>(&'a RefCell<RenderDataDrawingContext>);
                impl Drop for ResetRecorder<'_> {
                    fn drop(&mut self) {
                        if let Ok(mut recorder) = self.0.try_borrow_mut() {
                            recorder.reset();
                        }
                    }
                }
                let _reset = ResetRecorder(&self.recorder);
                let mut recorder = self.recorder.borrow_mut();
                {
                    let mut context = DrawingContext::new(&mut *recorder);
                    visual.render(&mut context);
                }
                recorder.get_render_results()
            };
            comp.set_draw_list(draw_list);

            visual.synchronize_composition_child_visuals();
        }
        let recalculate: Vec<Ref<Visual>> = self.recalculate_children.borrow().items.clone();
        for visual in &recalculate {
            if !self.dirty.borrow().contains(visual) {
                visual.synchronize_composition_child_visuals();
            }
        }
        self.dirty.borrow_mut().clear();
        self.recalculate_children.borrow_mut().clear();
        if let Some(root) = self.root.upgrade() {
            self.composition_target.set_size(root.client_size());
            self.composition_target.set_scaling(root.render_scaling());
        }

        let commit = self.compositor.request_composition_batch_commit_async();
        if !self.queued_scene_invalidation.get() {
            self.queued_scene_invalidation.set(true);
            // Updated hit-test information is available after full render
            let key = self.key;
            let dispatcher = self.compositor.dispatcher().clone();
            commit.rendered().on_completed(move || {
                dispatcher.post(
                    move || {
                        let renderer =
                            RENDERERS.with(|renderers| renderers.borrow().get(&key).and_then(Weak::upgrade));
                        if let Some(renderer) = renderer {
                            renderer.queued_scene_invalidation.set(false);
                            let size = renderer.root.upgrade().map_or(Size::default(), |root| root.client_size());
                            renderer.raise_scene_invalidated(&SceneInvalidatedEventArgs::new(Rect::from_size(size)));
                        }
                    },
                    DispatcherPriority::INPUT,
                );
            });
        }
    }

    fn raise_scene_invalidated(&self, e: &SceneInvalidatedEventArgs) {
        if self.scene_invalidated.is_empty() {
            return;
        }
        for (_, handler) in self.scene_invalidated.snapshot().iter() {
            handler(e);
        }
    }

    pub fn trigger_scene_invalidated_for_unit_tests(&self, rect: Rect) {
        self.raise_scene_invalidated(&SceneInvalidatedEventArgs::new(rect));
    }

    fn update(&self) {
        if self.updating.get() {
            return;
        }
        if !self.composition_target.is_enabled() {
            self.queued_update.set(false);
            return;
        }

        struct ResetUpdating<'a>(&'a Cell<bool>);
        impl Drop for ResetUpdating<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        self.updating.set(true);
        let _reset = ResetUpdating(&self.updating);
        self.update_core();
    }

    pub fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }
}

impl IRenderer for CompositingRenderer {
    fn diagnostics(&self) -> Rc<RendererDiagnostics> {
        self.diagnostics.clone()
    }

    fn scene_invalidated(&self, handler: Rc<dyn Fn(&SceneInvalidatedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.scene_invalidated.add(handler);
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.scene_invalidated.remove(token);
            }
        })
    }

    fn add_dirty(&self, visual: &Visual) {
        if self.is_disposed.get() {
            return;
        }
        if self.updating.get() {
            panic!("Visual was invalidated during the render pass");
        }
        self.dirty.borrow_mut().add(visual);
        self.queue_update();
    }

    fn recalculate_children(&self, visual: &Visual) {
        if self.is_disposed.get() {
            return;
        }
        if self.updating.get() {
            panic!("Visual was invalidated during the render pass");
        }
        self.recalculate_children.borrow_mut().add(visual);
        self.queue_update();
    }

    fn resized(&self, _size: Size) {}

    fn paint(&self, _rect: Rect) {
        if self.is_disposed.get() {
            return;
        }

        self.queue_update();
        self.composition_target.request_redraw();
        MediaContext::instance().immediate_render_requested(&self.compositor);
    }

    fn start(&self) {
        if self.is_disposed.get() {
            return;
        }

        self.composition_target.set_is_enabled(true);
        if !self.dirty.borrow().is_empty() || !self.recalculate_children.borrow().is_empty() {
            self.queue_update();
        }
    }

    fn stop(&self) {
        self.composition_target.set_is_enabled(false);
    }

    fn try_get_render_interface_feature(&self, feature_type: TypeId) -> Option<super::RenderInterfaceFeature> {
        self.compositor.try_get_render_interface_feature(feature_type)
    }

    fn compositor(&self) -> Option<Rc<Compositor>> {
        Some(self.compositor.clone())
    }

    fn dispose(&self) {
        if self.is_disposed.replace(true) {
            return;
        }
        self.dirty.borrow_mut().clear();
        self.recalculate_children.borrow_mut().clear();
        for (token, _) in self.scene_invalidated.snapshot().iter() {
            self.scene_invalidated.remove(*token);
        }
        self.stop();
        // The tree is released with the target: the root visual and the
        // target refer to each other.
        self.composition_target.set_root(None);
        self.composition_target.mark_disposed_out_of_band();
        MediaContext::instance().sync_dispose_composition_target(&self.compositor, self.composition_target.server());
        let key = self.key;
        let _ = RENDERERS.try_with(|renderers| {
            if let Ok(mut renderers) = renderers.try_borrow_mut() {
                renderers.remove(&key);
            }
        });
    }
}

impl IHitTester for CompositingRenderer {
    fn hit_test(&self, p: Point, root: &Visual, filter: Option<&dyn Fn(&Visual) -> bool>) -> Vec<Ref<Visual>> {
        self.hit_test_all::<PointCompositionHitTester>(&p, root, filter).into_iter().map(|(visual, _)| visual).collect()
    }

    fn hit_test_geometry(
        &self,
        geometry: &Geometry,
        root: &Visual,
        filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Vec<GeometryHitTestResult> {
        self.hit_test_all::<GeometryCompositionHitTester>(&geometry.to_ref(), root, filter)
            .into_iter()
            .map(|(visual, intersection)| GeometryHitTestResult::new(visual, intersection))
            .collect()
    }

    fn hit_test_first(&self, p: Point, root: &Visual, filter: Option<&dyn Fn(&Visual) -> bool>) -> Option<Ref<Visual>> {
        self.hit_test_first_core::<PointCompositionHitTester>(&p, root, filter).map(|(visual, _)| visual)
    }

    fn hit_test_first_geometry(
        &self,
        geometry: &Geometry,
        root: &Visual,
        filter: Option<&dyn Fn(&Visual) -> bool>,
    ) -> Option<GeometryHitTestResult> {
        self.hit_test_first_core::<GeometryCompositionHitTester>(&geometry.to_ref(), root, filter)
            .map(|(visual, intersection)| GeometryHitTestResult::new(visual, intersection))
    }
}
