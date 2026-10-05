use crate::gpu::ganesh::GaneshGrContext;
use crate::gpu::{ISkiaGpuRenderSession, ISkiaGpuRenderTarget, ISkiaGrContext, SkiaSurfaceOrigin};
use ferroui_base::platform::{PlatformRenderTargetState, RenderTargetSceneInfo};
use ferroui_opengl::gl_consts::{GL_FRAMEBUFFER_BINDING, GL_RGBA8};
use ferroui_opengl::surfaces::{
    IGlPlatformSurface, IGlPlatformSurfaceRenderTarget, IGlPlatformSurfaceRenderingSession,
};
use ferroui_opengl::IGlContext;
use skia_safe::gpu::gl::FramebufferInfo;
use skia_safe::gpu::{backend_render_targets, surfaces, BackendRenderTarget, SurfaceOrigin};
use skia_safe::{ColorType, PixelGeometry, Surface, SurfaceProps, SurfacePropsFlags};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The Skia render target of an OpenGL surface.
pub struct GlRenderTarget {
    gr_context: Rc<GaneshGrContext>,
    surface: Rc<dyn IGlPlatformSurfaceRenderTarget>,
}

impl GlRenderTarget {
    /// Creates the render target of `gl_surface` for `gl_context`.
    pub fn new(gr_context: Rc<GaneshGrContext>, gl_context: &Rc<dyn IGlContext>, gl_surface: &dyn IGlPlatformSurface) -> Self {
        let current = gl_context.ensure_current();
        let surface = gl_surface.create_gl_render_target(gl_context);
        current.dispose();
        Self { gr_context, surface }
    }
}

/// One frame rendered with Skia to an OpenGL framebuffer.
struct GlGpuSession {
    gr_context: Rc<GaneshGrContext>,
    backend_render_target: RefCell<Option<BackendRenderTarget>>,
    surface: RefCell<Option<Surface>>,
    gl_session: Rc<dyn IGlPlatformSurfaceRenderingSession>,
    surface_origin: SkiaSurfaceOrigin,
    disposed: Cell<bool>,
}

impl GlGpuSession {
    fn new(
        gr_context: Rc<GaneshGrContext>,
        backend_render_target: BackendRenderTarget,
        surface: Surface,
        gl_session: Rc<dyn IGlPlatformSurfaceRenderingSession>,
    ) -> Self {
        let surface_origin =
            if gl_session.is_y_flipped() { SkiaSurfaceOrigin::TopLeft } else { SkiaSurfaceOrigin::BottomLeft };
        Self {
            gr_context,
            backend_render_target: RefCell::new(Some(backend_render_target)),
            surface: RefCell::new(Some(surface)),
            gl_session,
            surface_origin,
            disposed: Cell::new(false),
        }
    }
}

impl ISkiaGpuRenderSession for GlGpuSession {
    fn gr_context(&self) -> Rc<dyn ISkiaGrContext> {
        self.gr_context.clone()
    }

    fn sk_surface(&self) -> Surface {
        self.surface.borrow().clone().expect("the OpenGL render session has been disposed")
    }

    fn scale_factor(&self) -> f64 {
        self.gl_session.scaling()
    }

    fn surface_origin(&self) -> SkiaSurfaceOrigin {
        self.surface_origin
    }

    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }

        if let Some(mut surface) = self.surface.borrow_mut().take() {
            if !self.gr_context.is_disposed() {
                self.gr_context.with_context(|context| {
                    context.flush_and_submit_surface(&mut surface, None);
                });
            }
        }
        self.backend_render_target.borrow_mut().take();
        self.gr_context.flush();
        self.gl_session.dispose();
    }
}

/// Disposes a platform session unless the Skia session took it over; what
/// the `finally` block of a managed implementation does.
struct SessionGuard {
    gl_session: Rc<dyn IGlPlatformSurfaceRenderingSession>,
    success: Cell<bool>,
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        if !self.success.get() {
            self.gl_session.dispose();
        }
    }
}

impl ISkiaGpuRenderTarget for GlRenderTarget {
    fn begin_rendering_session(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn ISkiaGpuRenderSession> {
        let gl_session = self.surface.begin_draw(scene_info);

        let guard = SessionGuard { gl_session: gl_session.clone(), success: Cell::new(false) };

        let disp = gl_session.context();
        let gl = disp.gl_interface();
        let fb = gl.get_integerv(GL_FRAMEBUFFER_BINDING);

        let size = gl_session.size();
        let color_type = ColorType::RGBA8888;
        let scaling = gl_session.scaling();
        if size.width <= 0 || size.height <= 0 || scaling < 0.0 {
            gl_session.dispose();
            panic!("Can't create drawing context for surface with {size} size and {scaling} scaling");
        }

        self.gr_context.reset_context();

        let (render_target, surface) = self.gr_context.with_context(|context| {
            let mut samples = disp.sample_count().max(0) as usize;
            let max_samples = context.max_surface_sample_count_for_color_type(color_type);
            if samples > max_samples {
                samples = max_samples;
            }

            let gl_info = FramebufferInfo { fboid: fb as u32, format: GL_RGBA8 as u32, ..Default::default() };
            let render_target = backend_render_targets::make_gl(
                (size.width, size.height),
                samples,
                disp.stencil_size().max(0) as usize,
                gl_info,
            );
            let surface_properties = SurfaceProps::new(SurfacePropsFlags::default(), PixelGeometry::RGBH);
            let surface = surfaces::wrap_backend_render_target(
                context,
                &render_target,
                if gl_session.is_y_flipped() { SurfaceOrigin::TopLeft } else { SurfaceOrigin::BottomLeft },
                color_type,
                None,
                Some(&surface_properties),
            );
            (render_target, surface)
        });

        let surface = surface.unwrap_or_else(|| panic!("Unable to create a Skia surface for the OpenGL framebuffer."));

        guard.success.set(true);

        Rc::new(GlGpuSession::new(self.gr_context.clone(), render_target, surface, gl_session))
    }

    fn state(&self) -> PlatformRenderTargetState {
        self.surface.state()
    }

    fn dispose(&self) {
        self.surface.dispose();
    }
}
