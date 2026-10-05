use super::web_render_target::{get_render_target, BrowserRenderTarget};
use super::{BrowserSharedRenderLoop, BrowserSurface};
use crate::browser_app_builder::BrowserRenderingMode;
use crate::interop::canvas_helper::CanvasSurface;
use crate::interop::JsObject;
use ferroui_base::media::MediaContext;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IOptionalFeatureProvider, IPlatformGraphics, IPlatformGraphicsContext, IPlatformGraphicsReadyStateFeature,
};
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{PixelSize, Size};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The surface of a view that is rendered through a render target of the
/// page (a WebGL context or a 2D canvas).
pub struct RenderTargetBrowserSurface {
    base: BrowserSurface,
    graphics: Rc<BrowserPlatformGraphics>,
}

impl RenderTargetBrowserSurface {
    fn new(js_surface: CanvasSurface) -> Rc<Self> {
        let target_id = js_surface.target_id();
        let graphics = BrowserPlatformGraphics::new(target_id);
        let gpu: Rc<dyn IPlatformGraphics> = graphics.clone();
        let compositor = Compositor::with_scheduler(
            BrowserSharedRenderLoop::render_loop(),
            Some(gpu),
            false,
            &MediaContext::instance().scheduler(),
            Dispatcher::ui_thread(),
            None,
            None,
        );

        let this = Rc::new(Self { base: BrowserSurface::new(js_surface, compositor), graphics });
        if let Some((w, h, s)) = this.base.initial_size() {
            this.on_size_changed(w, h, s);
        }
        this
    }

    /// Creates a canvas in `container` with a render target of the first of
    /// `modes` the browser supports, and the surface over it.
    pub fn create(container: &JsObject, modes: &[BrowserRenderingMode], top_level_id: i32) -> Rc<Self> {
        let modes: Vec<i32> = modes.iter().map(|m| *m as i32).collect();
        let js = CanvasSurface::create_render_target_surface(container, &modes, top_level_id);
        Self::new(js)
    }

    /// The compositor that renders to the surface.
    pub fn compositor(&self) -> Rc<Compositor> {
        self.base.compositor()
    }

    /// The scaling from logical units to device pixels.
    pub fn scaling(&self) -> f64 {
        self.base.scaling()
    }

    /// The size in logical units.
    pub fn client_size(&self) -> Size {
        self.base.client_size()
    }

    /// The size in device pixels.
    pub fn render_size(&self) -> PixelSize {
        self.base.render_size()
    }

    /// Whether the surface has a size it can be rendered at.
    pub fn is_valid(&self) -> bool {
        self.base.is_valid()
    }

    /// Subscribes to changes of the client size.
    pub fn size_changed(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.base.size_changed(handler)
    }

    /// Subscribes to changes of the scaling.
    pub fn scaling_changed(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.base.scaling_changed(handler)
    }

    /// The surfaces a render backend can draw to: the render target, once
    /// the page has created it.
    pub fn get_render_surfaces(&self) -> Vec<Rc<dyn IPlatformRenderSurface>> {
        match self.graphics.target() {
            Some(target) => vec![target.as_render_surface()],
            None => Vec::new(),
        }
    }

    /// The canvas changed its size or its scaling.
    pub fn on_size_changed(&self, pixel_width: f64, pixel_height: f64, dpr: f64) {
        self.graphics.canvas_size.set((PixelSize::new(pixel_width as i32, pixel_height as i32), dpr));
        self.base.on_size_changed(pixel_width, pixel_height, dpr);
    }

    /// Releases the canvas surface.
    ///
    /// The compositor is not told: it leaves the render loop by itself when
    /// the top-level that holds it is released.
    pub fn dispose(&self) {
        self.base.dispose();
    }
}

/// The platform graphics of one canvas: its render target, which may not
/// exist yet when the compositor is created.
struct BrowserPlatformGraphics {
    this: Weak<BrowserPlatformGraphics>,
    target_id: i32,
    target: RefCell<Option<Rc<dyn BrowserRenderTarget>>>,
    canvas_size: Cell<(PixelSize, f64)>,
}

impl BrowserPlatformGraphics {
    fn new(target_id: i32) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            target_id,
            target: RefCell::new(None),
            canvas_size: Cell::new((PixelSize::default(), 0.0)),
        })
    }

    fn target(&self) -> Option<Rc<dyn BrowserRenderTarget>> {
        if let Some(target) = self.target.borrow().clone() {
            return Some(target);
        }
        let this = self.this.clone();
        let target = get_render_target(
            self.target_id,
            Rc::new(move || this.upgrade().map_or((PixelSize::default(), 0.0), |this| this.canvas_size.get())),
        );
        *self.target.borrow_mut() = target.clone();
        target
    }

    fn uses_contexts(&self) -> bool {
        self.target().expect("the render target exists").platform_graphics_context().is_some()
    }
}

impl IPlatformGraphics for BrowserPlatformGraphics {
    fn uses_shared_context(&self) -> bool {
        self.uses_contexts()
    }

    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        panic!("Specified method is not supported.");
    }

    fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        match self.target().expect("the render target exists").platform_graphics_context() {
            Some(context) => context,
            None => panic!(
                "This platform graphics instance represents software rendering mode and cant create contexts, you are supposed to query IPlatformGraphicsReadyStateFeature to know this"
            ),
        }
    }

    fn as_feature_provider(&self) -> Option<&dyn IOptionalFeatureProvider> {
        Some(self)
    }
}

impl IOptionalFeatureProvider for BrowserPlatformGraphics {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IPlatformGraphicsReadyStateFeature>() {
            let this: Rc<dyn IPlatformGraphicsReadyStateFeature> = self.this.upgrade()?;
            return Some(Rc::new(this));
        }
        None
    }
}

impl IPlatformGraphicsReadyStateFeature for BrowserPlatformGraphics {
    fn is_ready(&self) -> bool {
        self.target().is_some() && self.canvas_size.get().0 != PixelSize::default()
    }

    fn uses_contexts(&self) -> bool {
        BrowserPlatformGraphics::uses_contexts(self)
    }
}
