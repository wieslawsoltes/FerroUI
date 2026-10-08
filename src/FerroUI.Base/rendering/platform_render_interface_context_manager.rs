use std::cell::RefCell;
use std::rc::{Rc, Weak};

use super::OwnedDisposable;
use crate::platform::surfaces::IPlatformRenderSurface;
use crate::platform::{
    IPlatformGraphics, IPlatformGraphicsContext, IPlatformGraphicsReadyStateFeature, IPlatformRenderInterface,
    IPlatformRenderInterfaceContext, IRenderTarget,
};
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::HandlerList;
use crate::{FerroLocator, LocatorExtensions};

/// Owns the backend render context of a renderer and recreates it (together
/// with the graphics context it is bound to) when it is lost.
///
/// Like the platform contracts it manages, it is confined to the thread it
/// was created on.
pub struct PlatformRenderInterfaceContextManager {
    weak_self: Weak<PlatformRenderInterfaceContextManager>,
    graphics: Option<Rc<dyn IPlatformGraphics>>,
    backend: RefCell<Option<Rc<dyn IPlatformRenderInterfaceContext>>>,
    gpu_context: RefCell<Option<OwnedDisposable<dyn IPlatformGraphicsContext>>>,
    ready_state_feature: Option<Rc<dyn IPlatformGraphicsReadyStateFeature>>,
    context_disposed: HandlerList<dyn Fn()>,
    context_created: HandlerList<dyn Fn(&Rc<dyn IPlatformRenderInterfaceContext>)>,
}

impl PlatformRenderInterfaceContextManager {
    pub fn new(graphics: Option<Rc<dyn IPlatformGraphics>>) -> Rc<PlatformRenderInterfaceContextManager> {
        let ready_state_feature = graphics
            .as_ref()
            .and_then(|graphics| graphics.as_feature_provider())
            .and_then(|features| features.try_get::<dyn IPlatformGraphicsReadyStateFeature>());

        Rc::new_cyclic(|weak_self| PlatformRenderInterfaceContextManager {
            weak_self: weak_self.clone(),
            graphics,
            backend: RefCell::new(None),
            gpu_context: RefCell::new(None),
            ready_state_feature,
            context_disposed: HandlerList::new(),
            context_created: HandlerList::new(),
        })
    }

    /// Raised after a lost graphics context has been released.
    pub fn context_disposed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.context_disposed.add(Rc::new(handler));
        let this = self.weak_self.clone();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.context_disposed.remove(token);
            }
        })
    }

    /// Raised after a backend context has been created.
    pub fn context_created(
        &self,
        handler: impl Fn(&Rc<dyn IPlatformRenderInterfaceContext>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.context_created.add(Rc::new(handler));
        let this = self.weak_self.clone();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.context_created.remove(token);
            }
        })
    }

    pub fn is_ready(&self) -> bool {
        self.ready_state_feature.as_ref().is_none_or(|feature| feature.is_ready())
    }

    /// Makes sure there is a backend context that is not lost.
    ///
    /// # Panics
    /// Panics when the platform graphics are not ready yet.
    pub fn ensure_valid_backend_context(&self) {
        if !self.is_ready() {
            panic!("Platform graphics isn't ready yet");
        }

        let gpu_context_is_lost = self.gpu_context().is_some_and(|context| context.is_lost());
        if self.backend.borrow().is_none() || gpu_context_is_lost {
            let backend = self.backend.borrow_mut().take();
            if let Some(backend) = backend {
                backend.dispose();
            }

            let gpu_context = self.gpu_context.borrow_mut().take();
            if let Some(mut gpu_context) = gpu_context {
                gpu_context.dispose();
                for (_, handler) in self.context_disposed.snapshot().iter() {
                    handler();
                }
            }

            if let Some(graphics) = &self.graphics {
                if self.ready_state_feature.as_ref().is_none_or(|feature| feature.uses_contexts()) {
                    let gpu_context = if graphics.uses_shared_context() {
                        OwnedDisposable::new(graphics.get_shared_context(), false)
                    } else {
                        OwnedDisposable::new(graphics.create_context(), true)
                    };
                    *self.gpu_context.borrow_mut() = Some(gpu_context);
                }
            }

            let backend = FerroLocator::current()
                .get_required_service::<dyn IPlatformRenderInterface>()
                .create_backend_context(self.gpu_context());
            *self.backend.borrow_mut() = Some(backend.clone());
            for (_, handler) in self.context_created.snapshot().iter() {
                handler(&backend);
            }
        }
    }

    /// The backend context; created (or recreated) when needed.
    pub fn value(&self) -> Rc<dyn IPlatformRenderInterfaceContext> {
        self.ensure_valid_backend_context();
        self.backend.borrow().clone().expect("the backend context was just ensured")
    }

    /// The graphics context the backend context is bound to, if any.
    pub fn gpu_context(&self) -> Option<Rc<dyn IPlatformGraphicsContext>> {
        self.gpu_context.borrow().as_ref().map(|context| context.value())
    }

    /// Makes the graphics context current. Disposing the result restores the
    /// previously current context.
    pub fn ensure_current(&self) -> Rc<dyn IDisposable> {
        self.ensure_valid_backend_context();
        match self.gpu_context() {
            Some(gpu_context) => gpu_context.ensure_current(),
            None => Disposable::empty(),
        }
    }

    pub fn create_render_target(&self, surfaces: &[Rc<dyn IPlatformRenderSurface>]) -> Rc<dyn IRenderTarget> {
        self.value().create_render_target(surfaces)
    }

    pub fn is_ready_to_create_render_target(&self, surfaces: &[Rc<dyn IPlatformRenderSurface>]) -> bool {
        let backend = self.backend.borrow().clone();
        match backend {
            None => self.is_ready(),
            Some(backend) => backend.is_ready_to_create_render_target(surfaces),
        }
    }

    /// Releases the backend context; the next use creates a new one.
    pub fn reset(&self) {
        let backend = self.backend.borrow_mut().take();
        if let Some(backend) = backend {
            backend.dispose();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use super::*;
    use crate::media::imaging::BitmapInterpolationMode;
    use crate::media::{FillRule, GeometryCombineMode};
    use crate::platform::{
        AlphaFormat, IBitmapImpl, IDrawingContextLayerImpl, IGeometryImpl, IOptionalFeatureProvider,
        IPlatformRenderInterfaceRegion, IRenderTargetBitmapImpl, IStreamGeometryImpl, IWriteableBitmapImpl,
        PixelFormat,
    };
    use crate::{PixelSize, Point, Rect, Vector};
    use std::any::{Any, TypeId};
    use std::cell::Cell;
    use std::io::Read;

    type Log = Rc<RefCell<Vec<String>>>;

    fn record(log: &Log, entry: impl Into<String>) {
        log.borrow_mut().push(entry.into());
    }

    fn take(log: &Log) -> Vec<String> {
        std::mem::take(&mut *log.borrow_mut())
    }

    struct GraphicsContext {
        id: i32,
        log: Log,
        is_lost: Cell<bool>,
    }

    impl IOptionalFeatureProvider for GraphicsContext {
        fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
            None
        }
    }

    impl IPlatformGraphicsContext for GraphicsContext {
        fn is_lost(&self) -> bool {
            self.is_lost.get()
        }

        fn ensure_current(&self) -> Rc<dyn IDisposable> {
            record(&self.log, format!("gpu{}.ensure_current", self.id));
            let log = self.log.clone();
            let id = self.id;
            Disposable::create(move || record(&log, format!("gpu{id}.restore")))
        }

        fn dispose(&self) {
            record(&self.log, format!("gpu{}.dispose", self.id));
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    struct ReadyState {
        is_ready: Cell<bool>,
        uses_contexts: Cell<bool>,
    }

    impl IPlatformGraphicsReadyStateFeature for ReadyState {
        fn is_ready(&self) -> bool {
            self.is_ready.get()
        }

        fn uses_contexts(&self) -> bool {
            self.uses_contexts.get()
        }
    }

    struct Graphics {
        log: Log,
        uses_shared_context: bool,
        shared: Rc<GraphicsContext>,
        created: RefCell<Vec<Rc<GraphicsContext>>>,
        ready_state: Option<Rc<ReadyState>>,
    }

    impl Graphics {
        fn new(log: &Log, uses_shared_context: bool, ready_state: Option<Rc<ReadyState>>) -> Rc<Graphics> {
            Rc::new(Graphics {
                log: log.clone(),
                uses_shared_context,
                shared: Rc::new(GraphicsContext { id: 0, log: log.clone(), is_lost: Cell::new(false) }),
                created: RefCell::new(Vec::new()),
                ready_state,
            })
        }
    }

    impl IOptionalFeatureProvider for Graphics {
        fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
            if feature_type == TypeId::of::<dyn IPlatformGraphicsReadyStateFeature>() {
                let feature: Rc<dyn IPlatformGraphicsReadyStateFeature> = self.ready_state.clone()?;
                return Some(Rc::new(feature));
            }
            None
        }
    }

    impl IPlatformGraphics for Graphics {
        fn uses_shared_context(&self) -> bool {
            self.uses_shared_context
        }

        fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
            let context = Rc::new(GraphicsContext {
                id: self.created.borrow().len() as i32 + 1,
                log: self.log.clone(),
                is_lost: Cell::new(false),
            });
            record(&self.log, format!("graphics.create_context -> gpu{}", context.id));
            self.created.borrow_mut().push(context.clone());
            context
        }

        fn get_shared_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
            record(&self.log, "graphics.get_shared_context");
            self.shared.clone()
        }

        fn as_feature_provider(&self) -> Option<&dyn IOptionalFeatureProvider> {
            Some(self)
        }
    }

    struct RenderTarget;

    impl IRenderTarget for RenderTarget {
        fn properties(&self) -> crate::platform::RenderTargetProperties {
            Default::default()
        }

        fn create_drawing_context(
            &self,
            _scene_info: &crate::platform::RenderTargetSceneInfo,
        ) -> (Box<dyn crate::platform::IDrawingContextImpl>, crate::platform::RenderTargetDrawingContextProperties)
        {
            unimplemented!()
        }

        fn dispose(&self) {}
    }

    struct BackendContext {
        id: i32,
        log: Log,
        ready_to_create_render_target: Cell<bool>,
    }

    impl IOptionalFeatureProvider for BackendContext {
        fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
            None
        }
    }

    impl IPlatformRenderInterfaceContext for BackendContext {
        fn create_render_target(&self, surfaces: &[Rc<dyn IPlatformRenderSurface>]) -> Rc<dyn IRenderTarget> {
            record(&self.log, format!("backend{}.create_render_target({})", self.id, surfaces.len()));
            Rc::new(RenderTarget)
        }

        fn create_offscreen_render_target(
            &self,
            _pixel_size: PixelSize,
            _scaling: Vector,
            _enable_text_antialiasing: bool,
        ) -> Rc<dyn IDrawingContextLayerImpl> {
            unimplemented!()
        }

        fn is_lost(&self) -> bool {
            false
        }

        fn max_offscreen_render_target_pixel_size(&self) -> Option<PixelSize> {
            None
        }

        fn is_ready_to_create_render_target(&self, _surfaces: &[Rc<dyn IPlatformRenderSurface>]) -> bool {
            self.ready_to_create_render_target.get()
        }

        fn dispose(&self) {
            record(&self.log, format!("backend{}.dispose", self.id));
        }
    }

    struct RenderInterface {
        log: Log,
        backends: RefCell<Vec<Rc<BackendContext>>>,
    }

    impl IPlatformRenderInterface for RenderInterface {
        fn create_backend_context(
            &self,
            graphics_api_context: Option<Rc<dyn IPlatformGraphicsContext>>,
        ) -> Rc<dyn IPlatformRenderInterfaceContext> {
            let backend = Rc::new(BackendContext {
                id: self.backends.borrow().len() as i32 + 1,
                log: self.log.clone(),
                ready_to_create_render_target: Cell::new(true),
            });
            let gpu = match &graphics_api_context {
                Some(context) => {
                    format!("gpu{}", context.as_any().downcast_ref::<GraphicsContext>().unwrap().id)
                }
                None => "software".to_string(),
            };
            record(&self.log, format!("create_backend_context({gpu}) -> backend{}", backend.id));
            self.backends.borrow_mut().push(backend.clone());
            backend
        }

        fn build_glyph_run_geometry(&self, _glyph_run: &crate::media::GlyphRun) -> Arc<dyn IGeometryImpl> {
            unimplemented!()
        }

        fn create_glyph_run(
            &self,
            _glyph_typeface: &Rc<crate::media::GlyphTypeface>,
            _font_rendering_em_size: f64,
            _glyph_infos: &[crate::media::text_formatting::GlyphInfo],
            _baseline_origin: Point,
        ) -> std::sync::Arc<dyn crate::platform::IGlyphRunImpl> {
            unimplemented!()
        }

        fn create_ellipse_geometry(&self, _rect: Rect) -> Arc<dyn IGeometryImpl> {
            unimplemented!()
        }

        fn create_line_geometry(&self, _p1: Point, _p2: Point) -> Arc<dyn IGeometryImpl> {
            unimplemented!()
        }

        fn create_rectangle_geometry(&self, _rect: Rect) -> Arc<dyn IGeometryImpl> {
            unimplemented!()
        }

        fn create_stream_geometry(&self) -> Arc<dyn IStreamGeometryImpl> {
            unimplemented!()
        }

        fn create_geometry_group(
            &self,
            _fill_rule: FillRule,
            _children: &[Arc<dyn IGeometryImpl>],
        ) -> Arc<dyn IGeometryImpl> {
            unimplemented!()
        }

        fn create_combined_geometry(
            &self,
            _combine_mode: GeometryCombineMode,
            _g1: Arc<dyn IGeometryImpl>,
            _g2: Arc<dyn IGeometryImpl>,
        ) -> Arc<dyn IGeometryImpl> {
            unimplemented!()
        }

        fn create_render_target_bitmap(&self, _size: PixelSize, _dpi: Vector) -> std::sync::Arc<dyn IRenderTargetBitmapImpl> {
            unimplemented!()
        }

        fn create_writeable_bitmap(
            &self,
            _size: PixelSize,
            _dpi: Vector,
            _format: PixelFormat,
            _alpha_format: AlphaFormat,
        ) -> std::sync::Arc<dyn IWriteableBitmapImpl> {
            unimplemented!()
        }

        fn load_bitmap_from_file(&self, _file_name: &str) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
            unimplemented!()
        }

        fn load_bitmap(&self, _stream: &mut dyn Read) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
            unimplemented!()
        }

        fn load_writeable_bitmap_to_width(
            &self,
            _stream: &mut dyn Read,
            _width: i32,
            _interpolation_mode: BitmapInterpolationMode,
        ) -> std::io::Result<std::sync::Arc<dyn IWriteableBitmapImpl>> {
            unimplemented!()
        }

        fn load_writeable_bitmap_to_height(
            &self,
            _stream: &mut dyn Read,
            _height: i32,
            _interpolation_mode: BitmapInterpolationMode,
        ) -> std::io::Result<std::sync::Arc<dyn IWriteableBitmapImpl>> {
            unimplemented!()
        }

        fn load_writeable_bitmap_from_file(&self, _file_name: &str) -> std::io::Result<std::sync::Arc<dyn IWriteableBitmapImpl>> {
            unimplemented!()
        }

        fn load_writeable_bitmap(&self, _stream: &mut dyn Read) -> std::io::Result<std::sync::Arc<dyn IWriteableBitmapImpl>> {
            unimplemented!()
        }

        fn load_bitmap_to_width(
            &self,
            _stream: &mut dyn Read,
            _width: i32,
            _interpolation_mode: BitmapInterpolationMode,
        ) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
            unimplemented!()
        }

        fn load_bitmap_to_height(
            &self,
            _stream: &mut dyn Read,
            _height: i32,
            _interpolation_mode: BitmapInterpolationMode,
        ) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
            unimplemented!()
        }

        fn resize_bitmap(
            &self,
            _bitmap_impl: &dyn IBitmapImpl,
            _destination_size: PixelSize,
            _interpolation_mode: BitmapInterpolationMode,
        ) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
            unimplemented!()
        }

        fn load_bitmap_from_pixels(
            &self,
            _format: PixelFormat,
            _alpha_format: AlphaFormat,
            _data: &[u8],
            _size: PixelSize,
            _dpi: Vector,
            _stride: i32,
        ) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
            unimplemented!()
        }

        fn supports_individual_round_rects(&self) -> bool {
            false
        }

        fn default_alpha_format(&self) -> AlphaFormat {
            unimplemented!()
        }

        fn default_pixel_format(&self) -> PixelFormat {
            unimplemented!()
        }

        fn is_supported_bitmap_pixel_format(&self, _format: PixelFormat) -> bool {
            false
        }

        fn supports_regions(&self) -> bool {
            false
        }

        fn create_region(&self) -> Rc<dyn IPlatformRenderInterfaceRegion> {
            unimplemented!()
        }
    }

    /// Registers a render interface for the test and restores the locator
    /// afterwards.
    struct Fixture {
        log: Log,
        render_interface: Rc<RenderInterface>,
        scope: Rc<dyn IDisposable>,
    }

    impl Fixture {
        fn new() -> Fixture {
            let log: Log = Rc::new(RefCell::new(Vec::new()));
            let scope = FerroLocator::enter_scope();
            let render_interface = Rc::new(RenderInterface { log: log.clone(), backends: RefCell::new(Vec::new()) });
            FerroLocator::current_mutable().bind::<dyn IPlatformRenderInterface>().to_constant(render_interface.clone());
            Fixture { log, render_interface, scope }
        }

        fn manager(&self, graphics: Option<Rc<Graphics>>) -> Rc<PlatformRenderInterfaceContextManager> {
            let manager =
                PlatformRenderInterfaceContextManager::new(graphics.map(|graphics| graphics as Rc<dyn IPlatformGraphics>));
            let log = self.log.clone();
            manager.context_disposed(move || record(&log, "ContextDisposed"));
            let log = self.log.clone();
            let render_interface = Rc::downgrade(&self.render_interface);
            manager.context_created(move |context| {
                let render_interface = render_interface.upgrade().unwrap();
                let backends = render_interface.backends.borrow();
                let created = backends.last().unwrap();
                assert!(std::ptr::eq(Rc::as_ptr(context) as *const (), Rc::as_ptr(created) as *const ()));
                record(&log, format!("ContextCreated(backend{})", created.id));
            });
            manager
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            self.scope.dispose();
        }
    }

    #[test]
    fn software_backend_is_created_once_without_a_graphics_context() {
        let fixture = Fixture::new();
        let manager = fixture.manager(None);
        assert!(manager.is_ready());
        assert!(manager.gpu_context().is_none());
        assert!(take(&fixture.log).is_empty());

        let value = manager.value();
        assert_eq!(take(&fixture.log), ["create_backend_context(software) -> backend1", "ContextCreated(backend1)"]);
        assert!(manager.gpu_context().is_none());

        assert!(Rc::ptr_eq(&value, &manager.value()));
        manager.ensure_valid_backend_context();
        // Without a graphics context there is nothing to make current.
        manager.ensure_current().dispose();
        assert!(take(&fixture.log).is_empty());
    }

    #[test]
    fn owned_graphics_context_is_recreated_when_lost() {
        let fixture = Fixture::new();
        let graphics = Graphics::new(&fixture.log, false, None);
        let manager = fixture.manager(Some(graphics.clone()));

        manager.ensure_valid_backend_context();
        assert_eq!(
            take(&fixture.log),
            [
                "graphics.create_context -> gpu1",
                "create_backend_context(gpu1) -> backend1",
                "ContextCreated(backend1)"
            ]
        );
        let gpu_context = manager.gpu_context().unwrap();
        assert!(std::ptr::eq(
            Rc::as_ptr(&gpu_context) as *const (),
            Rc::as_ptr(&graphics.created.borrow()[0]) as *const ()
        ));

        let current = manager.ensure_current();
        current.dispose();
        assert_eq!(take(&fixture.log), ["gpu1.ensure_current", "gpu1.restore"]);

        graphics.created.borrow()[0].is_lost.set(true);
        let value = manager.value();
        assert_eq!(
            take(&fixture.log),
            [
                "backend1.dispose",
                "gpu1.dispose",
                "ContextDisposed",
                "graphics.create_context -> gpu2",
                "create_backend_context(gpu2) -> backend2",
                "ContextCreated(backend2)"
            ]
        );
        assert!(std::ptr::eq(
            Rc::as_ptr(&value) as *const (),
            Rc::as_ptr(&fixture.render_interface.backends.borrow()[1]) as *const ()
        ));
    }

    #[test]
    fn shared_graphics_context_is_not_disposed() {
        let fixture = Fixture::new();
        let graphics = Graphics::new(&fixture.log, true, None);
        let manager = fixture.manager(Some(graphics.clone()));

        manager.ensure_valid_backend_context();
        assert_eq!(
            take(&fixture.log),
            ["graphics.get_shared_context", "create_backend_context(gpu0) -> backend1", "ContextCreated(backend1)"]
        );

        graphics.shared.is_lost.set(true);
        // The lost shared context is handed out again by the graphics, so
        // the backend is recreated on every use while it stays lost.
        manager.ensure_valid_backend_context();
        assert_eq!(
            take(&fixture.log),
            [
                "backend1.dispose",
                "ContextDisposed",
                "graphics.get_shared_context",
                "create_backend_context(gpu0) -> backend2",
                "ContextCreated(backend2)"
            ]
        );
    }

    #[test]
    fn reset_releases_the_backend_but_keeps_the_graphics_context() {
        let fixture = Fixture::new();
        let graphics = Graphics::new(&fixture.log, false, None);
        let manager = fixture.manager(Some(graphics));
        manager.ensure_valid_backend_context();
        take(&fixture.log);

        manager.reset();
        manager.reset();
        assert_eq!(take(&fixture.log), ["backend1.dispose"]);
        assert!(manager.gpu_context().is_some());

        // The graphics context is released and recreated with the backend.
        manager.ensure_valid_backend_context();
        assert_eq!(
            take(&fixture.log),
            [
                "gpu1.dispose",
                "ContextDisposed",
                "graphics.create_context -> gpu2",
                "create_backend_context(gpu2) -> backend2",
                "ContextCreated(backend2)"
            ]
        );
    }

    #[test]
    fn create_render_target_uses_the_backend() {
        let fixture = Fixture::new();
        let manager = fixture.manager(None);
        // Before a backend exists, readiness is the readiness of the graphics.
        assert!(manager.is_ready_to_create_render_target(&[]));
        assert!(take(&fixture.log).is_empty());

        manager.create_render_target(&[]);
        assert_eq!(
            take(&fixture.log),
            [
                "create_backend_context(software) -> backend1",
                "ContextCreated(backend1)",
                "backend1.create_render_target(0)"
            ]
        );

        fixture.render_interface.backends.borrow()[0].ready_to_create_render_target.set(false);
        assert!(!manager.is_ready_to_create_render_target(&[]));
    }

    #[test]
    fn ready_state_feature_controls_readiness_and_context_use() {
        let fixture = Fixture::new();
        let ready_state = Rc::new(ReadyState { is_ready: Cell::new(false), uses_contexts: Cell::new(false) });
        let graphics = Graphics::new(&fixture.log, false, Some(ready_state.clone()));
        let manager = fixture.manager(Some(graphics));

        assert!(!manager.is_ready());
        assert!(!manager.is_ready_to_create_render_target(&[]));
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| manager.ensure_valid_backend_context()));
        assert!(failed.is_err());
        assert!(take(&fixture.log).is_empty());

        ready_state.is_ready.set(true);
        assert!(manager.is_ready());
        manager.ensure_valid_backend_context();
        // The graphics do not use contexts: the backend is created without one.
        assert_eq!(take(&fixture.log), ["create_backend_context(software) -> backend1", "ContextCreated(backend1)"]);
        assert!(manager.gpu_context().is_none());

        manager.reset();
        ready_state.uses_contexts.set(true);
        manager.ensure_valid_backend_context();
        assert_eq!(
            take(&fixture.log),
            [
                "backend1.dispose",
                "graphics.create_context -> gpu1",
                "create_backend_context(gpu1) -> backend2",
                "ContextCreated(backend2)"
            ]
        );
    }

    #[test]
    fn event_subscriptions_can_be_disposed() {
        let fixture = Fixture::new();
        let manager = PlatformRenderInterfaceContextManager::new(None);
        let created = Rc::new(Cell::new(0));
        let c = created.clone();
        let subscription = manager.context_created(move |_| c.set(c.get() + 1));
        manager.ensure_valid_backend_context();
        assert_eq!(created.get(), 1);

        subscription.dispose();
        manager.reset();
        manager.ensure_valid_backend_context();
        assert_eq!(created.get(), 1);
        drop(fixture);
    }
}
