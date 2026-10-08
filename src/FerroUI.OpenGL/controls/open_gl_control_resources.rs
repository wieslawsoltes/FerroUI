use super::composition_open_gl_swapchain::CompositionOpenGlSwapchain;
use crate::composition::task_support::start;
use crate::composition::ICompositionGlContext;
use crate::gl_consts::*;
use crate::{GlProfileType, IGlContext, OpenGlException};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::rendering::composition::CompositionDrawingSurface;
use ferroui_base::threading::DispatcherTask;
use ferroui_base::PixelSize;
use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

/// The OpenGL objects of an OpenGL control: its framebuffer, its depth buffer and the swap
/// chain of the textures it draws into.
// The failures the original catches when it frees the framebuffer and the depth buffer of a
// context that may be lost are panics of the calls into the context here, and are not caught.
pub(crate) struct OpenGlControlBaseResources {
    depth_buffer: Cell<i32>,
    /// Shared with the frames in progress, which read it when they end.
    fbo: Rc<Cell<i32>>,
    depth_buffer_size: Cell<PixelSize>,
    surface: CompositionDrawingSurface,
    swapchain: CompositionOpenGlSwapchain,
    composition_context: Rc<dyn ICompositionGlContext>,
}

impl OpenGlControlBaseResources {
    pub(crate) fn new(context: Rc<dyn ICompositionGlContext>, surface: CompositionDrawingSurface) -> Self {
        let gl_context = context.gl_context();
        let current = gl_context.make_current();
        let fbo = gl_context.gl_interface().gen_framebuffer();
        current.dispose();
        Self {
            depth_buffer: Cell::new(0),
            fbo: Rc::new(Cell::new(fbo)),
            depth_buffer_size: Cell::new(PixelSize::default()),
            surface: surface.clone(),
            swapchain: CompositionOpenGlSwapchain::new(context.clone(), surface),
            composition_context: context,
        }
    }

    pub(crate) fn fbo(&self) -> i32 {
        self.fbo.get()
    }

    pub(crate) fn surface(&self) -> &CompositionDrawingSurface {
        &self.surface
    }

    pub(crate) fn context(&self) -> Rc<dyn IGlContext> {
        self.composition_context.gl_context()
    }

    pub(crate) fn is_lost(&self) -> bool {
        !self.composition_context.is_valid_for_interop()
    }

    fn update_depth_renderbuffer(&self, size: PixelSize) {
        if size == self.depth_buffer_size.get() && self.depth_buffer.get() != 0 {
            return;
        }

        let context = self.context();
        let gl = context.gl_interface();
        let old_render_buffer = gl.get_integerv(GL_RENDERBUFFER_BINDING);
        if self.depth_buffer.get() != 0 {
            gl.delete_renderbuffer(self.depth_buffer.get());
        }

        self.depth_buffer.set(gl.gen_renderbuffer());
        gl.bind_renderbuffer(GL_RENDERBUFFER, self.depth_buffer.get());
        gl.renderbuffer_storage(
            GL_RENDERBUFFER,
            if context.version().type_() == GlProfileType::OpenGLES { GL_DEPTH_COMPONENT16 } else { GL_DEPTH_COMPONENT },
            size.width,
            size.height,
        );
        gl.framebuffer_renderbuffer(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_RENDERBUFFER, self.depth_buffer.get());
        gl.bind_renderbuffer(GL_RENDERBUFFER, old_render_buffer);
        self.depth_buffer_size.set(size);
    }

    /// Starts a frame of `size`: the context is made current and the framebuffer is bound
    /// to the texture of the frame. Disposing the result presents the frame and restores
    /// the context. A framebuffer that cannot be configured is the error (the exception of
    /// the original); the frame is discarded and the context restored before it is returned.
    pub(crate) fn begin_draw(&self, size: PixelSize) -> Result<Rc<dyn IDisposable>, OpenGlException> {
        let context = self.context();
        let restore_context = context.ensure_current();

        let gl = context.gl_interface();
        gl.bind_framebuffer(GL_FRAMEBUFFER, self.fbo.get());
        self.update_depth_renderbuffer(size);

        let (lease, texture) = self.swapchain.begin_draw(size);
        gl.framebuffer_texture_2d(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, texture.target, texture.texture_id, 0);

        let status = gl.check_framebuffer_status(GL_FRAMEBUFFER);
        if status != GL_FRAMEBUFFER_COMPLETE {
            let code = gl.get_error();
            if let Some(logger) = Logger::try_get(LogEventLevel::Error, "OpenGL") {
                let source: &dyn Any = &"OpenGlControlBase";
                logger.log_with_values(Some(source), "Unable to configure OpenGL FBO: {code}", &[&code]);
            }
            // The frame wasn't rendered, so it shouldn't reach the surface
            lease.discard();
            restore_context.dispose();
            return Err(OpenGlException::get_formatted_exception_for_code("Unable to configure OpenGL FBO", code));
        }

        let fbo = self.fbo.clone();
        Ok(Disposable::create(move || {
            // The texture should be unbound from user framebuffers before it's presented.
            // User code could have bound another framebuffer, so rebind ours first
            let gl_done = context.gl_interface();
            gl_done.bind_framebuffer(GL_FRAMEBUFFER, fbo.get());
            gl_done.framebuffer_texture_2d(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, texture.target, 0, 0);
            lease.dispose();
            restore_context.dispose();
        }))
    }

    pub(crate) fn dispose_async(&self) -> DispatcherTask<()> {
        if !self.is_lost() {
            let context = self.context();
            let current = context.make_current();
            let gl = context.gl_interface();
            if self.fbo.get() != 0 {
                gl.delete_framebuffer(self.fbo.get());
            }
            self.fbo.set(0);
            if self.depth_buffer.get() != 0 {
                gl.delete_renderbuffer(self.depth_buffer.get());
            }
            self.depth_buffer.set(0);
            current.dispose();
        }
        self.surface.dispose();

        let swapchain_disposal = self.swapchain.dispose_async();
        let composition_context = self.composition_context.clone();
        start(async move {
            let _ = swapchain_disposal.await;
            let _ = composition_context.dispose_async().await;
        })
    }
}
