use crate::presentation_source::{IRendererFactory, ITopLevelRenderer, RenderSurfaces};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::rendering::composition::{CompositionTransparencyLevel, Compositor};
use ferroui_base::rendering::{IPresentationSource, IRenderer, RendererDiagnostics, SceneInvalidatedEventArgs};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{FerroLocator, Rect, Ref, Size, Visual};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A renderer that draws nothing and records what the top-level layer asks
/// of it.
pub struct NullRenderer {
    this: Weak<NullRenderer>,
    diagnostics: Rc<RendererDiagnostics>,
    scene_invalidated: HandlerList<dyn Fn(&SceneInvalidatedEventArgs)>,
    root: RefCell<Option<Ref<Visual>>>,
    transparency_level: Cell<CompositionTransparencyLevel>,
    platform_specific_scene_info: RefCell<Option<std::sync::Arc<dyn Any + Send + Sync>>>,
    is_started: Cell<bool>,
    is_disposed: Cell<bool>,
    resized: RefCell<Vec<Size>>,
    painted: RefCell<Vec<Rect>>,
    dirty: Cell<usize>,
}

impl NullRenderer {
    pub fn new() -> Rc<NullRenderer> {
        Rc::new_cyclic(|this| NullRenderer {
            this: this.clone(),
            diagnostics: RendererDiagnostics::new(),
            scene_invalidated: HandlerList::new(),
            root: RefCell::new(None),
            transparency_level: Cell::new(CompositionTransparencyLevel::None),
            platform_specific_scene_info: RefCell::new(None),
            is_started: Cell::new(false),
            is_disposed: Cell::new(false),
            resized: RefCell::new(Vec::new()),
            painted: RefCell::new(Vec::new()),
            dirty: Cell::new(0),
        })
    }

    /// The factory that creates a null renderer for every presentation
    /// source.
    pub fn factory() -> Rc<dyn IRendererFactory> {
        Rc::new(
            |_root: Rc<dyn IPresentationSource>, _compositor: Option<Rc<Compositor>>, _surfaces: RenderSurfaces| {
                let renderer: Rc<dyn ITopLevelRenderer> = NullRenderer::new();
                renderer
            },
        )
    }

    /// Registers [`factory`](Self::factory) in the current service locator.
    pub fn register() {
        FerroLocator::current_mutable().bind::<dyn IRendererFactory>().to_constant(Self::factory());
    }

    /// Raises the scene invalidated event.
    pub fn raise_scene_invalidated(&self, dirty_rect: Rect) {
        let e = SceneInvalidatedEventArgs::new(dirty_rect);
        for (_, handler) in self.scene_invalidated.snapshot().iter() {
            handler(&e);
        }
    }

    /// The root set by the presentation source.
    pub fn root(&self) -> Option<Ref<Visual>> {
        self.root.borrow().clone()
    }

    pub fn transparency_level(&self) -> CompositionTransparencyLevel {
        self.transparency_level.get()
    }

    pub fn platform_specific_scene_info(&self) -> Option<std::sync::Arc<dyn Any + Send + Sync>> {
        self.platform_specific_scene_info.borrow().clone()
    }

    /// Whether the renderer has been started and not stopped since.
    pub fn is_started(&self) -> bool {
        self.is_started.get()
    }

    pub fn is_disposed(&self) -> bool {
        self.is_disposed.get()
    }

    /// The sizes passed to `resized`, in call order.
    pub fn resized_calls(&self) -> Vec<Size> {
        self.resized.borrow().clone()
    }

    /// The rectangles passed to `paint`, in call order.
    pub fn paint_calls(&self) -> Vec<Rect> {
        self.painted.borrow().clone()
    }

    /// The number of `add_dirty` calls.
    pub fn add_dirty_count(&self) -> usize {
        self.dirty.get()
    }
}

impl IRenderer for NullRenderer {
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

    fn add_dirty(&self, _visual: &Visual) {
        self.dirty.set(self.dirty.get() + 1);
    }

    fn recalculate_children(&self, _visual: &Visual) {}

    fn resized(&self, size: Size) {
        self.resized.borrow_mut().push(size);
    }

    fn paint(&self, rect: Rect) {
        self.painted.borrow_mut().push(rect);
    }

    fn start(&self) {
        self.is_started.set(true);
    }

    fn stop(&self) {
        self.is_started.set(false);
    }

    fn try_get_render_interface_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }

    fn dispose(&self) {
        self.is_disposed.set(true);
        *self.root.borrow_mut() = None;
    }
}

impl ITopLevelRenderer for NullRenderer {
    fn set_root(&self, root: Option<Ref<Visual>>) {
        if self.is_disposed.get() {
            return;
        }
        *self.root.borrow_mut() = root;
    }

    fn set_transparency_level(&self, level: CompositionTransparencyLevel) {
        self.transparency_level.set(level);
    }

    fn set_platform_specific_scene_info(&self, info: Option<std::sync::Arc<dyn Any + Send + Sync>>) {
        *self.platform_specific_scene_info.borrow_mut() = info;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
