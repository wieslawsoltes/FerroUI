use super::{
    FboSkiaSurface, GlRenderTarget, GlSkiaExternalObjectsFeature, GlSkiaSharedTextureForComposition,
    IGlSkiaSpecificOptionsFeature,
};
use crate::gpu::ganesh::GaneshGrContext;
use crate::gpu::{
    ISkiaGpu, ISkiaGpuRenderSession, ISkiaGpuRenderTarget, ISkiaGrContext, ISkiaSurface, ScopedGrContext,
    SkiaSurfaceOrigin,
};
use crate::skia_options::SkiaOptions;
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IExternalObjectsHandleWrapRenderInterfaceContextFeature, IExternalObjectsRenderInterfaceContextFeature,
    IOptionalFeatureProvider, IPlatformGraphicsContext,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::PixelSize;
use ferroui_opengl::gl_consts::*;
use ferroui_opengl::surfaces::{try_get_gl_surface, IGlPlatformSurface, IGlPlatformSurfaceRenderTarget};
use ferroui_opengl::{
    GlProfileType, GlVersion, ICompositionImportableOpenGlSharedTexture, IGlContext,
    IGlContextExternalObjectsFeature, IGlPlatformSurfaceRenderTargetFactory,
    IOpenGlTextureSharingRenderInterfaceContextFeature,
};
use skia_safe::gpu::gl::Interface;
use skia_safe::gpu::{direct_contexts, ContextOptions};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// A Skia GPU that renders with a platform OpenGL context through Ganesh.
pub struct GlSkiaGpu {
    gr_context: Rc<GaneshGrContext>,
    gl_context: Rc<dyn IGlContext>,
    post_dispose_callbacks: RefCell<Vec<Box<dyn FnOnce()>>>,
    can_create_surfaces: Cell<Option<bool>>,
    external_objects_feature: Rc<GlSkiaExternalObjectsFeature>,
    /// Marks the shared textures made for this GPU: what a thread that
    /// cannot reach the context of such a texture compares instead of
    /// asking the contexts whether they share.
    share_group: Arc<()>,
    this: Weak<GlSkiaGpu>,
}

impl GlSkiaGpu {
    /// Creates a Skia GPU for an OpenGL context.
    ///
    /// # Panics
    /// Panics when Skia cannot create a context for the OpenGL context.
    pub fn new(context: Rc<dyn IGlContext>, max_resource_bytes: Option<i64>, use_stencil_buffers: Option<bool>) -> Rc<Self> {
        let current = context.ensure_current();

        let features: &dyn IOptionalFeatureProvider = &*context;
        let iface = if features
            .try_get::<dyn IGlSkiaSpecificOptionsFeature>()
            .is_some_and(|skia_options| skia_options.use_native_skia_gr_gl_interface())
        {
            Interface::new_native()
        } else {
            // Skia assembles the interface of the flavour (OpenGL or OpenGL
            // ES) that the context reports through its version string.
            let gl = context.gl_interface();
            Interface::new_load_with(|proc| gl.get_proc_address(proc))
        };
        let iface = iface.unwrap_or_else(|| panic!("Unable to create a Skia OpenGL interface for the context."));

        let avoid_stencil_buffers = SkiaOptions::should_avoid_stencil_buffers(use_stencil_buffers);
        let mut options = ContextOptions::default();
        options.avoid_stencil_buffers = avoid_stencil_buffers;
        let gr_context = direct_contexts::make_gl(iface, &options)
            .map(GaneshGrContext::new)
            .unwrap_or_else(|| panic!("Unable to create a Skia GPU context from the OpenGL context."));
        if let Some(max_resource_bytes) = max_resource_bytes {
            gr_context.set_resource_cache_limit(max_resource_bytes);
        }
        let gr_context = Rc::new(gr_context);

        let external_objects = features.try_get::<dyn IGlContextExternalObjectsFeature>();
        let share_group = Arc::new(());
        let external_objects_feature = GlSkiaExternalObjectsFeature::new(
            gr_context.clone(),
            context.clone(),
            share_group.clone(),
            external_objects,
        );

        current.dispose();

        Rc::new_cyclic(|this| Self {
            gr_context,
            gl_context: context,
            post_dispose_callbacks: RefCell::new(Vec::new()),
            can_create_surfaces: Cell::new(None),
            external_objects_feature,
            share_group,
            this: this.clone(),
        })
    }

    /// The Ganesh context of the GPU.
    pub fn gr_context(&self) -> Rc<GaneshGrContext> {
        self.gr_context.clone()
    }

    /// The OpenGL context the GPU renders with.
    pub fn gl_context(&self) -> Rc<dyn IGlContext> {
        self.gl_context.clone()
    }

    /// Whether [`create_shared_context`](Self::create_shared_context) is
    /// supported.
    pub fn can_create_shared_context(&self) -> bool {
        self.gl_context.can_create_shared_context()
    }

    /// Creates an OpenGL context that shares its objects with the one of the
    /// GPU.
    pub fn create_shared_context(&self, preferred_versions: Option<&[GlVersion]>) -> Option<Rc<dyn IGlContext>> {
        self.gl_context.create_shared_context(preferred_versions)
    }

    /// Creates a texture of `context`, a context of the share group of the
    /// GPU, for a drawing surface of the compositor.
    ///
    /// The texture belongs to the thread of `context`, which calls this;
    /// the thread that renders imports it.
    ///
    /// # Panics
    /// Panics when the contexts do not share their objects (the exception
    /// of the original).
    pub fn create_shared_texture_for_composition(
        &self,
        context: &Rc<dyn IGlContext>,
        size: PixelSize,
    ) -> Arc<dyn ICompositionImportableOpenGlSharedTexture> {
        if !context.is_shared_with(&*self.gl_context) {
            panic!("Contexts do not belong to the same share group");
        }

        let current = context.ensure_current();

        let gl = context.gl_interface();
        let old_texture = gl.get_integerv(GL_TEXTURE_BINDING_2D);
        let tex = gl.gen_texture();

        let format = if context.version().type_() == GlProfileType::OpenGLES && context.version().major() == 2 {
            GL_RGBA
        } else {
            GL_RGBA8
        };

        gl.bind_texture(GL_TEXTURE_2D, tex);
        // SAFETY: a null data pointer makes OpenGL allocate the storage
        // without reading client memory.
        unsafe {
            gl.tex_image_2d(
                GL_TEXTURE_2D,
                0,
                format,
                size.width,
                size.height,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                std::ptr::null(),
            );
        }

        gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
        gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
        gl.bind_texture(GL_TEXTURE_2D, old_texture);

        current.dispose();

        Arc::new(GlSkiaSharedTextureForComposition::new(context.clone(), tex, format, size, self.share_group.clone()))
    }

    /// Runs `dispose` after the Skia context has been released, when the
    /// GPU is disposed.
    pub fn add_post_dispose(&self, dispose: impl FnOnce() + 'static) {
        self.post_dispose_callbacks.borrow_mut().push(Box::new(dispose));
    }

    fn render_target_factory(&self) -> Option<Rc<dyn IGlPlatformSurfaceRenderTargetFactory>> {
        let features: &dyn IOptionalFeatureProvider = &*self.gl_context;
        features.try_get::<dyn IGlPlatformSurfaceRenderTargetFactory>()
    }
}

/// Renders to a surface that is not an OpenGL surface itself through the
/// render target factory of the context.
///
/// It holds nothing but the shared handle of the surface, and lives for the creation of
/// one render target on the thread that renders.
struct SurfaceWrapper {
    surface: std::sync::Arc<dyn IPlatformRenderSurface>,
}

impl IPlatformRenderSurface for SurfaceWrapper {
    fn is_ready(&self) -> bool {
        self.surface.is_ready()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IGlPlatformSurface for SurfaceWrapper {
    fn create_gl_render_target(&self, context: &Rc<dyn IGlContext>) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
        let features: &dyn IOptionalFeatureProvider = &**context;
        let feature = features
            .try_get::<dyn IGlPlatformSurfaceRenderTargetFactory>()
            .expect("the context has a render target factory");
        feature.create_render_target(context, &self.surface)
    }
}

impl IOpenGlTextureSharingRenderInterfaceContextFeature for GlSkiaGpu {
    fn can_create_shared_context(&self) -> bool {
        GlSkiaGpu::can_create_shared_context(self)
    }

    fn create_shared_context(&self, preferred_versions: Option<&[GlVersion]>) -> Option<Rc<dyn IGlContext>> {
        GlSkiaGpu::create_shared_context(self, preferred_versions)
    }

    fn create_shared_texture_for_composition(
        &self,
        context: &Rc<dyn IGlContext>,
        size: PixelSize,
    ) -> Arc<dyn ICompositionImportableOpenGlSharedTexture> {
        GlSkiaGpu::create_shared_texture_for_composition(self, context, size)
    }
}

impl IOptionalFeatureProvider for GlSkiaGpu {
    /// The GPU itself as the texture sharing feature, its external objects
    /// feature, and the handle wrapping feature of the OpenGL context.
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IOpenGlTextureSharingRenderInterfaceContextFeature>() {
            let this: Rc<dyn IOpenGlTextureSharingRenderInterfaceContextFeature> = self.this.upgrade()?;
            return Some(Rc::new(this));
        }
        if feature_type == TypeId::of::<dyn IExternalObjectsRenderInterfaceContextFeature>() {
            let feature: Rc<dyn IExternalObjectsRenderInterfaceContextFeature> = self.external_objects_feature.clone();
            return Some(Rc::new(feature));
        }
        if feature_type == TypeId::of::<dyn IExternalObjectsHandleWrapRenderInterfaceContextFeature>() {
            return self.gl_context.try_get_feature(feature_type);
        }
        None
    }
}

impl IPlatformGraphicsContext for GlSkiaGpu {
    fn is_lost(&self) -> bool {
        self.gl_context.is_lost()
    }

    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        self.gl_context.ensure_current()
    }

    fn dispose(&self) {
        self.gr_context.dispose(!self.gl_context.is_lost());

        let callbacks = std::mem::take(&mut *self.post_dispose_callbacks.borrow_mut());
        for cb in callbacks {
            cb();
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ISkiaGpu for GlSkiaGpu {
    fn platform_graphics_context(&self) -> Option<Rc<dyn IPlatformGraphicsContext>> {
        Some(self.gl_context.clone())
    }

    fn try_create_render_target(
        &self,
        surfaces: &[std::sync::Arc<dyn IPlatformRenderSurface>],
    ) -> Option<Rc<dyn ISkiaGpuRenderTarget>> {
        let custom_render_target_factory = self.render_target_factory();
        for surface in surfaces {
            if custom_render_target_factory
                .as_ref()
                .is_some_and(|factory| factory.can_render_to_surface(&self.gl_context, surface))
            {
                let wrapper = SurfaceWrapper { surface: surface.clone() };
                return Some(Rc::new(GlRenderTarget::new(self.gr_context.clone(), &self.gl_context, &wrapper)));
            }
            if let Some(gl_surface) = try_get_gl_surface(&**surface) {
                return Some(Rc::new(GlRenderTarget::new(self.gr_context.clone(), &self.gl_context, &*gl_surface)));
            }
        }

        None
    }

    fn is_ready_to_create_render_target(&self, surfaces: &[std::sync::Arc<dyn IPlatformRenderSurface>]) -> bool {
        let custom_render_target_factory = self.render_target_factory();
        for surface in surfaces {
            if custom_render_target_factory
                .as_ref()
                .is_some_and(|factory| factory.can_render_to_surface(&self.gl_context, surface))
                || try_get_gl_surface(&**surface).is_some()
            {
                return surface.is_ready();
            }
        }

        false
    }

    fn try_create_surface(
        &self,
        size: PixelSize,
        session: Option<&Rc<dyn ISkiaGpuRenderSession>>,
    ) -> Option<Rc<dyn ISkiaSurface>> {
        // Only windows platform needs our FBO trickery
        if !cfg!(target_os = "windows") {
            return None;
        }

        // Blit feature requires glBlitFramebuffer
        if !self.gl_context.gl_interface().is_blit_framebuffer_available() {
            return None;
        }

        let size = PixelSize::new(size.width.max(1), size.height.max(1));
        if self.can_create_surfaces.get() == Some(false) {
            return None;
        }
        let gpu = self.this.upgrade()?;
        match FboSkiaSurface::new(
            gpu,
            self.gr_context.clone(),
            self.gl_context.clone(),
            size,
            session.map_or(SkiaSurfaceOrigin::TopLeft, |session| session.surface_origin()),
        ) {
            Ok(surface) => {
                self.can_create_surfaces.set(Some(true));
                Some(Rc::new(surface))
            }
            Err(_) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                    logger.log(Some(self as &dyn Any), "Unable to create a Skia-compatible FBO manually");
                }
                if self.can_create_surfaces.get().is_none() {
                    self.can_create_surfaces.set(Some(false));
                }
                None
            }
        }
    }

    fn try_get_gr_context(&self) -> Option<ScopedGrContext> {
        let context: Rc<dyn ISkiaGrContext> = self.gr_context.clone();
        Some(ScopedGrContext::new(context, Some(self.ensure_current())))
    }
}
