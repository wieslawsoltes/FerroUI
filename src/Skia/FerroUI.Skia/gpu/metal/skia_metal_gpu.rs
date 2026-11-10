use super::{AutoReleasePool, SkiaMetalExternalObjectsFeature};
#[cfg(ferro_skia_apple_ganesh)]
use crate::gpu::ganesh::GaneshGrContext;
#[cfg(not(ferro_skia_apple_ganesh))]
use crate::gpu::graphite::GraphiteGrContext;
use crate::gpu::{
    ISkiaGpu, ISkiaGpuRenderSession, ISkiaGpuRenderTarget, ISkiaGrContext, ISkiaSurface, ScopedGrContext,
    SkiaSurfaceOrigin,
};
use crate::metal::{
    try_get_metal_surface, IMetalDevice, IMetalExternalObjectsFeature, IMetalPlatformSurfaceRenderTarget,
    IMetalPlatformSurfaceRenderingSession,
};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IExternalObjectsHandleWrapRenderInterfaceContextFeature, IExternalObjectsRenderInterfaceContextFeature,
    IOptionalFeatureProvider, IPlatformGraphicsContext, PlatformRenderTargetState, RenderTargetSceneInfo,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::PixelSize;
#[cfg(ferro_skia_apple_ganesh)]
use crate::skia_options::SkiaOptions;
#[cfg(ferro_skia_apple_ganesh)]
use skia_safe::gpu::{
    backend_render_targets, direct_contexts, mtl, surfaces, BackendRenderTarget, ContextOptions, SurfaceOrigin,
};
#[cfg(not(ferro_skia_apple_ganesh))]
use skia_safe::gpu::graphite::mtl::{backend_textures, context_factory, BackendContext};
#[cfg(not(ferro_skia_apple_ganesh))]
use skia_safe::gpu::graphite::surfaces;
use skia_safe::{ColorType, Surface};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

/// The Skia context of a Metal GPU: Graphite, or Ganesh in the configuration
/// `ferro_skia_apple_ganesh` (the original's `GRContext`).
#[cfg(not(ferro_skia_apple_ganesh))]
pub(super) type MetalGrContext = GraphiteGrContext;
/// The Skia context of a Metal GPU: Graphite, or Ganesh in the configuration
/// `ferro_skia_apple_ganesh` (the original's `GRContext`).
#[cfg(ferro_skia_apple_ganesh)]
pub(super) type MetalGrContext = GaneshGrContext;

/// The Skia context shared by a Metal GPU and its render targets; empty
/// once the GPU has been disposed.
pub(super) type SharedContext = Rc<RefCell<Option<Rc<MetalGrContext>>>>;

/// A Skia GPU that renders with a platform Metal device through Graphite
/// (Ganesh in the configuration `ferro_skia_apple_ganesh`).
pub struct SkiaMetalGpu {
    context: SharedContext,
    device: Rc<dyn IMetalDevice>,
    external_objects: Option<Rc<SkiaMetalExternalObjectsFeature>>,
}

impl SkiaMetalGpu {
    /// Creates a Skia GPU for a Metal device.
    ///
    /// `max_resource_bytes` and `use_stencil_buffers` are accepted for parity
    /// with the other GPUs; Graphite manages its resource budget itself and
    /// has no stencil-based path renderer to opt into.
    ///
    /// # Panics
    /// Panics when Skia cannot create a context for the device.
    #[cfg(not(ferro_skia_apple_ganesh))]
    pub fn new(device: Rc<dyn IMetalDevice>, max_resource_bytes: Option<i64>, _use_stencil_buffers: Option<bool>) -> Rc<Self> {
        // SAFETY: the device contract hands out valid `id<MTLDevice>` and
        // `id<MTLCommandQueue>` handles; Skia retains both for the lifetime
        // of the backend context.
        let backend_context = unsafe { BackendContext::new(device.device(), device.command_queue()) };

        let context = context_factory::make_metal(&backend_context, &ordered_recordings_options())
            .and_then(GraphiteGrContext::new)
            .unwrap_or_else(|| panic!("Unable to create a Skia GPU context from the Metal device."));

        if let Some(max_resource_bytes) = max_resource_bytes {
            context.set_resource_cache_limit(max_resource_bytes);
        }
        let queue = device.command_queue() as usize;
        context.set_frame_completion_handler(Box::new(move |completion| {
            // SAFETY: the Metal GPU owns the device/queue through context
            // submission. The native command buffer retains its queue.
            unsafe { super::frame_completion::schedule(queue as *mut _, completion); }
        }));
        let context: SharedContext = Rc::new(RefCell::new(Some(Rc::new(context))));

        let features: &dyn IOptionalFeatureProvider = &*device;
        let external_objects = features
            .try_get::<dyn IMetalExternalObjectsFeature>()
            .map(|external_objects| Rc::new(SkiaMetalExternalObjectsFeature::new(context.clone(), external_objects)));

        Rc::new(Self { context, device, external_objects })
    }

    /// Creates a Skia GPU for a Metal device on Ganesh (the configuration
    /// `ferro_skia_apple_ganesh`), as the original does: the stencil option
    /// and the limit of the resource cache go to the context.
    ///
    /// # Panics
    /// Panics when Skia cannot create a context for the device.
    #[cfg(ferro_skia_apple_ganesh)]
    pub fn new(device: Rc<dyn IMetalDevice>, max_resource_bytes: Option<i64>, use_stencil_buffers: Option<bool>) -> Rc<Self> {
        let avoid_stencil_buffers = SkiaOptions::should_avoid_stencil_buffers(use_stencil_buffers);
        // SAFETY: the device contract hands out valid `id<MTLDevice>` and
        // `id<MTLCommandQueue>` handles; the backend context retains both
        // until it is dropped, and the direct context takes references of
        // its own.
        let backend_context = unsafe { mtl::BackendContext::new(device.device() as _, device.command_queue() as _) };
        let mut options = ContextOptions::default();
        options.avoid_stencil_buffers = avoid_stencil_buffers;
        let context = direct_contexts::make_metal(&backend_context, &options)
            .map(GaneshGrContext::new)
            .unwrap_or_else(|| panic!("Unable to create GRContext from Metal device."));
        let context = Rc::new(context);
        let shared: SharedContext = Rc::new(RefCell::new(Some(context.clone())));

        let features: &dyn IOptionalFeatureProvider = &*device;
        let external_objects = features
            .try_get::<dyn IMetalExternalObjectsFeature>()
            .map(|external_objects| Rc::new(SkiaMetalExternalObjectsFeature::new(shared.clone(), external_objects)));
        if let Some(max_resource_bytes) = max_resource_bytes {
            context.set_resource_cache_limit(max_resource_bytes);
        }

        Rc::new(Self { context: shared, device, external_objects })
    }

    /// The Skia context of the GPU.
    ///
    /// # Panics
    /// Panics when the GPU has been disposed.
    pub fn gr_context(&self) -> Rc<MetalGrContext> {
        self.context.borrow().clone().expect("SkiaMetalGpu has been disposed")
    }
}

impl IOptionalFeatureProvider for SkiaMetalGpu {
    /// The external objects feature of the GPU, when the device has one,
    /// and the handle wrapping feature of the device.
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type==TypeId::of::<dyn IMetalDevice>() {return Some(Rc::new(self.device.clone()));}
        if feature_type == TypeId::of::<dyn IExternalObjectsHandleWrapRenderInterfaceContextFeature>() {
            return self.device.try_get_feature(feature_type);
        }
        if feature_type == TypeId::of::<dyn IExternalObjectsRenderInterfaceContextFeature>() {
            let feature: Rc<dyn IExternalObjectsRenderInterfaceContextFeature> = self.external_objects.clone()?;
            return Some(Rc::new(feature));
        }
        None
    }
}

impl IPlatformGraphicsContext for SkiaMetalGpu {
    fn is_lost(&self) -> bool {
        false
    }

    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        self.device.ensure_current()
    }

    fn dispose(&self) {
        if let Some(context) = self.context.borrow_mut().take() {
            #[cfg(not(ferro_skia_apple_ganesh))]
            context.dispose();
            // The device is never lost, so the resources are freed with it.
            #[cfg(ferro_skia_apple_ganesh)]
            context.dispose(true);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ISkiaGpu for SkiaMetalGpu {
    fn platform_graphics_context(&self) -> Option<Rc<dyn IPlatformGraphicsContext>> {
        Some(self.device.clone())
    }

    fn try_create_render_target(
        &self,
        surfaces: &[std::sync::Arc<dyn IPlatformRenderSurface>],
    ) -> Option<Rc<dyn ISkiaGpuRenderTarget>> {
        for surface in surfaces {
            if let Some(metal_surface) = try_get_metal_surface(&**surface) {
                let target = metal_surface.create_metal_render_target(self.device.clone());
                return Some(Rc::new(SkiaMetalRenderTarget::new(self.context.clone(), target)));
            }
        }

        None
    }

    fn is_ready_to_create_render_target(&self, surfaces: &[std::sync::Arc<dyn IPlatformRenderSurface>]) -> bool {
        for surface in surfaces {
            if try_get_metal_surface(&**surface).is_some() {
                return surface.is_ready();
            }
        }

        false
    }

    /// A frame layer that can draw a frame while the one before it is
    /// presented (`gpu/graphite/pipelined_layer.rs`), unless
    /// [`SkiaOptions::frame_pipelining`](crate::SkiaOptions::frame_pipelining)
    /// is off; `None` otherwise, which gives the layer a surface of the
    /// context.
    ///
    /// Only the layer a render target context creates carries a session:
    /// that is the frame layer of the compositor. Every other surface is
    /// created without one and stays a surface of the context.
    #[cfg(not(ferro_skia_apple_ganesh))]
    fn try_create_surface(
        &self,
        size: PixelSize,
        session: Option<&Rc<dyn ISkiaGpuRenderSession>>,
        surface_props: &skia_safe::SurfaceProps,
    ) -> Option<Rc<dyn ISkiaSurface>> {
        session?;
        let context = self.context.borrow().clone()?;
        let options = crate::SkiaOptions::current();
        let layer = crate::gpu::graphite::PipelinedLayerSurface::try_new(
            &context,
            self.device.device(),
            size,
            surface_props,
            options.frame_pipelining,
            options.effective_frame_pipelining_bands(),
        )?;
        Some(Rc::new(layer))
    }

    #[cfg(ferro_skia_apple_ganesh)]
    fn try_create_surface(
        &self,
        _size: PixelSize,
        _session: Option<&Rc<dyn ISkiaGpuRenderSession>>,
        _surface_props: &skia_safe::SurfaceProps,
    ) -> Option<Rc<dyn ISkiaSurface>> {
        None
    }

    fn try_get_gr_context(&self) -> Option<ScopedGrContext> {
        let context: Rc<dyn ISkiaGrContext> = self.gr_context();
        Some(ScopedGrContext::new(context, Some(self.ensure_current())))
    }
}

/// The Skia render target of a Metal surface.
pub struct SkiaMetalRenderTarget {
    context: SharedContext,
    target: RefCell<Option<Rc<dyn IMetalPlatformSurfaceRenderTarget>>>,
}

impl SkiaMetalRenderTarget {
    fn new(context: SharedContext, target: Rc<dyn IMetalPlatformSurfaceRenderTarget>) -> Self {
        Self { context, target: RefCell::new(Some(target)) }
    }
}

impl ISkiaGpuRenderTarget for SkiaMetalRenderTarget {
    #[cfg(not(ferro_skia_apple_ganesh))]
    fn begin_rendering_session(
        &self,
        _scene_info: &RenderTargetSceneInfo,
        surface_props: &skia_safe::SurfaceProps,
    ) -> Rc<dyn ISkiaGpuRenderSession> {
        let target = self.target.borrow().clone().expect("SkiaMetalRenderTarget has been disposed");
        let context = self.context.borrow().clone().expect("SkiaMetalGpu has been disposed");

        let session = target.begin_rendering();
        let size = session.size();

        // SAFETY: the session keeps the `id<MTLTexture>` it hands out alive
        // until it is disposed, which happens after the surface wrapping the
        // texture is released (see `SkiaMetalRenderSession::dispose`).
        let backend_texture = unsafe { backend_textures::make_metal((size.width, size.height), session.texture()) };

        let surface = context.with_recorder(|recorder| {
            surfaces::wrap_backend_texture(recorder, &backend_texture, ColorType::BGRA8888, None, Some(surface_props))
        });

        let Some(surface) = surface else {
            session.dispose();
            panic!("Unable to create a Skia surface for the Metal texture.");
        };

        Rc::new(SkiaMetalRenderSession::new(context, surface, session))
    }

    /// The texture of the session as a Ganesh render target with the origin
    /// the session reports (the configuration `ferro_skia_apple_ganesh`), as
    /// the original does.
    #[cfg(ferro_skia_apple_ganesh)]
    fn begin_rendering_session(
        &self,
        _scene_info: &RenderTargetSceneInfo,
        surface_props: &skia_safe::SurfaceProps,
    ) -> Rc<dyn ISkiaGpuRenderSession> {
        // The original: "TODO: use expectedPixelSize".
        let target = self.target.borrow().clone().expect("SkiaMetalRenderTarget has been disposed");
        let context = self.context.borrow().clone().expect("SkiaMetalGpu has been disposed");

        let session = target.begin_rendering();
        let size = session.size();

        // SAFETY: the session keeps the `id<MTLTexture>` it hands out alive
        // until it is disposed, which happens after the surface and the
        // render target wrapping the texture are released (see
        // `SkiaMetalRenderSession::dispose`).
        let texture_info = unsafe { mtl::TextureInfo::new(session.texture() as _) };
        let backend_target = backend_render_targets::make_mtl((size.width, size.height), &texture_info);

        let origin = if session.is_y_flipped() { SurfaceOrigin::BottomLeft } else { SurfaceOrigin::TopLeft };
        let surface = context.with_context(|context| {
            surfaces::wrap_backend_render_target(
                context,
                &backend_target,
                origin,
                ColorType::BGRA8888,
                None,
                Some(surface_props),
            )
        });

        let Some(surface) = surface else {
            drop(backend_target);
            session.dispose();
            panic!("Unable to create a Skia surface for the Metal texture.");
        };

        Rc::new(SkiaMetalRenderSession::new(context, surface, session, backend_target))
    }

    fn state(&self) -> PlatformRenderTargetState {
        match &*self.target.borrow() {
            Some(target) => target.state(),
            None => PlatformRenderTargetState::DISPOSED,
        }
    }

    fn dispose(&self) {
        if let Some(target) = self.target.borrow_mut().take() {
            target.dispose();
        }
    }
}

/// One frame rendered with Skia to a Metal texture.
pub struct SkiaMetalRenderSession {
    context: Rc<MetalGrContext>,
    surface: RefCell<Option<Surface>>,
    session: RefCell<Option<Rc<dyn IMetalPlatformSurfaceRenderingSession>>>,
    /// The texture of the session as a Ganesh render target, released after
    /// the surface and the session.
    #[cfg(ferro_skia_apple_ganesh)]
    backend_target: RefCell<Option<BackendRenderTarget>>,
    scale_factor: f64,
    is_y_flipped: bool,
    auto_release_pool: AutoReleasePool,
}

impl SkiaMetalRenderSession {
    fn new(
        context: Rc<MetalGrContext>,
        surface: Surface,
        session: Rc<dyn IMetalPlatformSurfaceRenderingSession>,
        #[cfg(ferro_skia_apple_ganesh)] backend_target: BackendRenderTarget,
    ) -> Self {
        Self {
            auto_release_pool: AutoReleasePool::new(),
            context,
            surface: RefCell::new(Some(surface)),
            scale_factor: session.scaling(),
            is_y_flipped: session.is_y_flipped(),
            session: RefCell::new(Some(session)),
            #[cfg(ferro_skia_apple_ganesh)]
            backend_target: RefCell::new(Some(backend_target)),
        }
    }
}

impl ISkiaGpuRenderSession for SkiaMetalRenderSession {
    fn gr_context(&self) -> Rc<dyn ISkiaGrContext> {
        self.context.clone()
    }

    fn sk_surface(&self) -> Surface {
        self.surface.borrow().clone().expect("SkiaMetalRenderSession has been disposed")
    }

    fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    fn surface_origin(&self) -> SkiaSurfaceOrigin {
        if self.is_y_flipped {
            SkiaSurfaceOrigin::BottomLeft
        } else {
            SkiaSurfaceOrigin::TopLeft
        }
    }

    fn dispose(&self) {
        if !self.context.is_disposed() {
            self.context.flush();
        }

        self.surface.borrow_mut().take();

        if let Some(session) = self.session.borrow_mut().take() {
            session.dispose();
        }

        #[cfg(ferro_skia_apple_ganesh)]
        self.backend_target.borrow_mut().take();

        self.auto_release_pool.dispose();
    }
}

/// The options of the window's Graphite context: recordings are inserted in the order they were
/// snapped (desktop-performance.md section 9.17). Without it Graphite cannot keep its glyph atlas
/// from one recording to the next and evicts it at every snap (`Recorder::snap`), so every frame
/// rasterizes and uploads every glyph it draws again; Skia's own Metal window sets it too
/// (`tools/window/GraphiteNativeMetalWindowContext.mm`). The window inserts its recordings in
/// order already. skia-safe has no setter for the option.
#[cfg(not(ferro_skia_apple_ganesh))]
fn ordered_recordings_options() -> skia_safe::gpu::graphite::ContextOptions {
    let mut options = skia_safe::gpu::graphite::ContextOptions::new();
    // SAFETY: `Handle<N>` is `repr(transparent)` over `UnsafeCell<N>`, itself transparent over the
    // native `skgpu::graphite::ContextOptions`, whose `fRequireOrderedRecordings` is a plain bool.
    unsafe {
        let native = &mut *(&mut options as *mut skia_safe::gpu::graphite::ContextOptions)
            .cast::<skia_bindings::skgpu_graphite_ContextOptions>();
        native.fRequireOrderedRecordings = true;
    }
    options
}
