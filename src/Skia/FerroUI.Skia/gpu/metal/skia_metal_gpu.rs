use super::AutoReleasePool;
use crate::gpu::graphite::GraphiteGrContext;
use crate::gpu::{
    ISkiaGpu, ISkiaGpuRenderSession, ISkiaGpuRenderTarget, ISkiaGrContext, ISkiaSurface, ScopedGrContext,
    SkiaSurfaceOrigin,
};
use crate::metal::{
    try_get_metal_surface, IMetalDevice, IMetalPlatformSurfaceRenderTarget, IMetalPlatformSurfaceRenderingSession,
};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IOptionalFeatureProvider, IPlatformGraphicsContext, PlatformRenderTargetState, RenderTargetSceneInfo,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::PixelSize;
use skia_safe::gpu::graphite::mtl::{backend_textures, context_factory, BackendContext};
use skia_safe::gpu::graphite::surfaces;
use skia_safe::{ColorType, Surface};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::Rc;

/// The Graphite context shared by a Metal GPU and its render targets; empty
/// once the GPU has been disposed.
type SharedContext = Rc<RefCell<Option<Rc<GraphiteGrContext>>>>;

/// A Skia GPU that renders with a platform Metal device through Graphite.
pub struct SkiaMetalGpu {
    context: SharedContext,
    device: Rc<dyn IMetalDevice>,
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
    pub fn new(device: Rc<dyn IMetalDevice>, max_resource_bytes: Option<i64>, _use_stencil_buffers: Option<bool>) -> Rc<Self> {
        // SAFETY: the device contract hands out valid `id<MTLDevice>` and
        // `id<MTLCommandQueue>` handles; Skia retains both for the lifetime
        // of the backend context.
        let backend_context = unsafe { BackendContext::new(device.device(), device.command_queue()) };

        let context = context_factory::make_metal(&backend_context, None)
            .and_then(GraphiteGrContext::new)
            .unwrap_or_else(|| panic!("Unable to create a Skia GPU context from the Metal device."));

        if let Some(max_resource_bytes) = max_resource_bytes {
            context.set_resource_cache_limit(max_resource_bytes);
        }

        Rc::new(Self { context: Rc::new(RefCell::new(Some(Rc::new(context)))), device })
    }

    /// The Graphite context of the GPU.
    ///
    /// # Panics
    /// Panics when the GPU has been disposed.
    pub fn gr_context(&self) -> Rc<GraphiteGrContext> {
        self.context.borrow().clone().expect("SkiaMetalGpu has been disposed")
    }
}

impl IOptionalFeatureProvider for SkiaMetalGpu {
    fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
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
            context.dispose();
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
        surfaces: &[Rc<dyn IPlatformRenderSurface>],
    ) -> Option<Rc<dyn ISkiaGpuRenderTarget>> {
        for surface in surfaces {
            if let Some(metal_surface) = try_get_metal_surface(&**surface) {
                let target = metal_surface.create_metal_render_target(self.device.clone());
                return Some(Rc::new(SkiaMetalRenderTarget::new(self.context.clone(), target)));
            }
        }

        None
    }

    fn is_ready_to_create_render_target(&self, surfaces: &[Rc<dyn IPlatformRenderSurface>]) -> bool {
        for surface in surfaces {
            if try_get_metal_surface(&**surface).is_some() {
                return surface.is_ready();
            }
        }

        false
    }

    fn try_create_surface(
        &self,
        _size: PixelSize,
        _session: Option<&Rc<dyn ISkiaGpuRenderSession>>,
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
    fn begin_rendering_session(&self, _scene_info: &RenderTargetSceneInfo) -> Rc<dyn ISkiaGpuRenderSession> {
        let target = self.target.borrow().clone().expect("SkiaMetalRenderTarget has been disposed");
        let context = self.context.borrow().clone().expect("SkiaMetalGpu has been disposed");

        let session = target.begin_rendering();
        let size = session.size();

        // SAFETY: the session keeps the `id<MTLTexture>` it hands out alive
        // until it is disposed, which happens after the surface wrapping the
        // texture is released (see `SkiaMetalRenderSession::dispose`).
        let backend_texture = unsafe { backend_textures::make_metal((size.width, size.height), session.texture()) };

        let surface = context.with_recorder(|recorder| {
            surfaces::wrap_backend_texture(recorder, &backend_texture, ColorType::BGRA8888, None, None)
        });

        let Some(surface) = surface else {
            session.dispose();
            panic!("Unable to create a Skia surface for the Metal texture.");
        };

        Rc::new(SkiaMetalRenderSession::new(context, surface, session))
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
    context: Rc<GraphiteGrContext>,
    surface: RefCell<Option<Surface>>,
    session: RefCell<Option<Rc<dyn IMetalPlatformSurfaceRenderingSession>>>,
    scale_factor: f64,
    is_y_flipped: bool,
    auto_release_pool: AutoReleasePool,
}

impl SkiaMetalRenderSession {
    fn new(
        context: Rc<GraphiteGrContext>,
        surface: Surface,
        session: Rc<dyn IMetalPlatformSurfaceRenderingSession>,
    ) -> Self {
        Self {
            auto_release_pool: AutoReleasePool::new(),
            context,
            surface: RefCell::new(Some(surface)),
            scale_factor: session.scaling(),
            is_y_flipped: session.is_y_flipped(),
            session: RefCell::new(Some(session)),
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

        self.auto_release_pool.dispose();
    }
}
