use super::GlSkiaGpu;
use crate::gpu::ganesh::GaneshGrContext;
use crate::gpu::{ISkiaGrContext, ISkiaSurface, SkiaSurfaceOrigin};
use ferroui_base::PixelSize;
use ferroui_opengl::gl_consts::*;
use ferroui_opengl::{GlProfileType, IGlContext, OpenGlException};
use skia_safe::gpu::gl::FramebufferInfo;
use skia_safe::gpu::{backend_render_targets, surfaces, SurfaceOrigin};
use skia_safe::{Canvas, Color, ColorType, PixelGeometry, Surface, SurfaceProps, SurfacePropsFlags};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An offscreen surface backed by a framebuffer object that the backend
/// creates itself, so that its contents can be copied with a framebuffer
/// blit.
pub struct FboSkiaSurface {
    gpu: Rc<GlSkiaGpu>,
    gr_context: Rc<GaneshGrContext>,
    gl_context: Rc<dyn IGlContext>,
    pixel_size: PixelSize,
    fbo: Cell<i32>,
    depth_stencil: Cell<i32>,
    texture: Cell<i32>,
    surface: RefCell<Option<Surface>>,
    can_blit: bool,
}

const TRUE_FALSE: [bool; 2] = [true, false];

impl FboSkiaSurface {
    /// Creates the framebuffer object and the Skia surface over it. The
    /// context must be current.
    pub fn new(
        gpu: Rc<GlSkiaGpu>,
        gr_context: Rc<GaneshGrContext>,
        gl_context: Rc<dyn IGlContext>,
        pixel_size: PixelSize,
        surface_origin: SkiaSurfaceOrigin,
    ) -> Result<Self, OpenGlException> {
        let internal_format = if gl_context.version().type_() == GlProfileType::OpenGLES { GL_RGBA } else { GL_RGBA8 };
        let gl = gl_context.gl_interface();

        // Save old bindings
        let old_fbo = gl.get_integerv(GL_FRAMEBUFFER_BINDING);
        let old_renderbuffer = gl.get_integerv(GL_RENDERBUFFER_BINDING);
        let old_texture = gl.get_integerv(GL_TEXTURE_BINDING_2D);

        // Generate FBO
        let fbo = gl.gen_framebuffer();
        gl.bind_framebuffer(GL_FRAMEBUFFER, fbo);

        // Create a texture to render into
        let texture = gl.gen_texture();
        gl.bind_texture(GL_TEXTURE_2D, texture);
        // SAFETY: a null data pointer makes OpenGL allocate the storage
        // without reading client memory.
        unsafe {
            gl.tex_image_2d(
                GL_TEXTURE_2D,
                0,
                internal_format,
                pixel_size.width,
                pixel_size.height,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                std::ptr::null(),
            );
        }
        gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
        gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
        gl.framebuffer_texture_2d(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, texture, 0);

        let mut success = false;
        let mut depth_stencil = 0;
        for use_stencil8 in TRUE_FALSE {
            depth_stencil = gl.gen_renderbuffer();
            gl.bind_renderbuffer(GL_RENDERBUFFER, depth_stencil);

            if use_stencil8 {
                gl.renderbuffer_storage(GL_RENDERBUFFER, GL_STENCIL_INDEX8, pixel_size.width, pixel_size.height);
                gl.framebuffer_renderbuffer(GL_FRAMEBUFFER, GL_STENCIL_ATTACHMENT, GL_RENDERBUFFER, depth_stencil);
            } else {
                gl.renderbuffer_storage(GL_RENDERBUFFER, GL_DEPTH24_STENCIL8, pixel_size.width, pixel_size.height);
                gl.framebuffer_renderbuffer(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_RENDERBUFFER, depth_stencil);
                gl.framebuffer_renderbuffer(GL_FRAMEBUFFER, GL_STENCIL_ATTACHMENT, GL_RENDERBUFFER, depth_stencil);
            }

            let status = gl.check_framebuffer_status(GL_FRAMEBUFFER);
            if status == GL_FRAMEBUFFER_COMPLETE {
                success = true;
                break;
            } else {
                gl.bind_renderbuffer(GL_RENDERBUFFER, old_renderbuffer);
                gl.delete_renderbuffer(depth_stencil);
            }
        }

        gl.bind_renderbuffer(GL_RENDERBUFFER, old_renderbuffer);
        gl.bind_texture(GL_TEXTURE_2D, old_texture);
        gl.bind_framebuffer(GL_FRAMEBUFFER, old_fbo);

        if !success {
            gl.delete_framebuffer(fbo);
            gl.delete_texture(texture);
            return Err(OpenGlException::new("Unable to create FBO with stencil"));
        }

        let target = backend_render_targets::make_gl(
            (pixel_size.width, pixel_size.height),
            0,
            8,
            FramebufferInfo { fboid: fbo as u32, format: GL_RGBA8 as u32, ..Default::default() },
        );
        let properties = SurfaceProps::new(SurfacePropsFlags::default(), PixelGeometry::RGBH);
        let surface = gr_context.with_context(|context| {
            surfaces::wrap_backend_render_target(
                context,
                &target,
                match surface_origin {
                    SkiaSurfaceOrigin::TopLeft => SurfaceOrigin::TopLeft,
                    SkiaSurfaceOrigin::BottomLeft => SurfaceOrigin::BottomLeft,
                },
                ColorType::RGBA8888,
                None,
                Some(&properties),
            )
        });
        let can_blit = gl.is_blit_framebuffer_available();

        Ok(Self {
            gpu,
            gr_context,
            gl_context,
            pixel_size,
            fbo: Cell::new(fbo),
            depth_stencil: Cell::new(depth_stencil),
            texture: Cell::new(texture),
            surface: RefCell::new(surface),
            can_blit,
        })
    }
}

impl ISkiaSurface for FboSkiaSurface {
    fn surface(&self) -> Surface {
        match self.surface.borrow().as_ref() {
            Some(surface) => surface.clone(),
            None => panic!("Cannot access a disposed object: FboSkiaSurface"),
        }
    }

    fn can_blit(&self) -> bool {
        self.can_blit
    }

    fn blit(&self, canvas: &Canvas) {
        // This should set the render target as the current FBO
        // which is definitely not the best method, but it works
        canvas.clear(Color::TRANSPARENT);
        self.gr_context.flush();

        let gl = self.gl_context.gl_interface();
        let old_read = gl.get_integerv(GL_READ_FRAMEBUFFER_BINDING);
        gl.bind_framebuffer(GL_READ_FRAMEBUFFER, self.fbo.get());
        gl.blit_framebuffer(
            0,
            0,
            self.pixel_size.width,
            self.pixel_size.height,
            0,
            0,
            self.pixel_size.width,
            self.pixel_size.height,
            GL_COLOR_BUFFER_BIT,
            GL_LINEAR,
        );
        gl.bind_framebuffer(GL_READ_FRAMEBUFFER, old_read);
    }

    fn dispose(&self) {
        if self.gl_context.is_lost() {
            if let Some(surface) = self.surface.borrow_mut().take() {
                // The Skia surface has to be released _after_ the Skia context was
                // abandoned, otherwise it will try to do OpenGL calls without a proper context
                self.gpu.add_post_dispose(move || drop(surface));
            }
        } else {
            let current = self.gl_context.ensure_current();
            self.surface.borrow_mut().take();
            let gl = self.gl_context.gl_interface();
            if self.fbo.get() != 0 {
                gl.delete_framebuffer(self.fbo.get());
                gl.delete_texture(self.texture.get());
                gl.delete_renderbuffer(self.depth_stencil.get());
            }
            current.dispose();
        }

        self.fbo.set(0);
        self.texture.set(0);
        self.depth_stencil.set(0);
    }
}
