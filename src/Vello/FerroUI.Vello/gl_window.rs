//! The windows of a platform whose graphics are OpenGL, drawn in the CPU mode
//! and presented through the context of the platform.
//!
//! A platform that renders with OpenGL hands its windows out as surfaces of
//! OpenGL (GLX and EGL on X11, EGL on Wayland, ANGLE on Direct3D 11 or WGL on
//! Windows), and on Windows the composition modes of the window present only
//! what is drawn through those surfaces: a framebuffer drawn with GDI does
//! not show in a window of WinUI composition or DirectComposition. Where the
//! backend has no device of its own for the GPU modes (`gpu::create_gpu`
//! gave none: no adapter that is hardware), the scene of a window is drawn
//! by the CPU renderer into a frame the render target keeps, as for a
//! framebuffer (`FramebufferRenderTarget`), and each frame is presented
//! through the context of the platform: what changed is uploaded into a
//! texture of the context, and the texture is copied into the framebuffer
//! of the surface (`glBlitFramebuffer`, OpenGL 3 and OpenGL ES 3), which the
//! session presents.

use crate::i_drawable_bitmap_impl::IDrawableBitmapImpl;
use crate::surface_render_target::{SurfaceChange, SurfaceRenderTarget, SurfaceRenderTargetCreateInfo};
use crate::vello_options::VelloRenderingMode;
use crate::vello_platform::VelloPlatform;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IBitmapImpl, IDrawingContextImpl, IOptionalFeatureProvider, IPlatformGraphicsContext, IRenderTarget,
    PlatformRenderTargetState, RenderTargetDrawingContextProperties, RenderTargetProperties, RenderTargetSceneInfo,
};
use ferroui_base::PixelSize;
use ferroui_opengl::gl_consts::{
    GL_COLOR_ATTACHMENT0, GL_COLOR_BUFFER_BIT, GL_DRAW_FRAMEBUFFER, GL_FRAMEBUFFER, GL_FRAMEBUFFER_BINDING, GL_NEAREST,
    GL_READ_FRAMEBUFFER, GL_RGBA, GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_TEXTURE_MIN_FILTER, GL_UNPACK_ALIGNMENT,
    GL_UNPACK_ROW_LENGTH, GL_UNSIGNED_BYTE,
};
use ferroui_opengl::surfaces::{try_get_gl_surface, IGlPlatformSurfaceRenderTarget};
use ferroui_opengl::{GlInterface, IGlContext, IGlPlatformSurfaceRenderTargetFactory};
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::Arc;

/// What presents the windows of a platform whose graphics are OpenGL (see
/// the module).
pub struct VelloGlWindows {
    gl_context: Rc<dyn IGlContext>,
}

impl VelloGlWindows {
    /// The windows of the platform of `graphics_context`, when it is a
    /// context of OpenGL that copies between framebuffers.
    pub fn try_new(graphics_context: &Rc<dyn IPlatformGraphicsContext>) -> Option<Rc<Self>> {
        let features: &dyn IOptionalFeatureProvider = &**graphics_context;
        let gl_context = features.try_get::<dyn IGlContext>()?;
        let gl = gl_context.gl_interface();
        (gl.is_blit_framebuffer_available() && gl.is_tex_sub_image_2d_available())
            .then(|| Rc::new(Self { gl_context }))
    }

    /// The render target of the first surface the context renders to: one
    /// its render target factory takes (the surfaces of the composition
    /// modes of Windows), or a surface of OpenGL.
    pub fn try_create_render_target(
        &self,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
        rendering_modes: &[VelloRenderingMode],
    ) -> Option<Rc<dyn IRenderTarget>> {
        let target = gl_render_target(&self.gl_context, surfaces)?;
        Some(Rc::new(VelloGlRenderTarget::new(self.gl_context.clone(), target, rendering_modes)))
    }
}

/// The render target of the first surface a context of OpenGL renders to:
/// one its render target factory takes (the surfaces of the composition
/// modes of Windows), or a surface of OpenGL; as the Skia backend finds it.
pub(crate) fn gl_render_target(
    gl_context: &Rc<dyn IGlContext>,
    surfaces: &[Arc<dyn IPlatformRenderSurface>],
) -> Option<Rc<dyn IGlPlatformSurfaceRenderTarget>> {
    let features: &dyn IOptionalFeatureProvider = &**gl_context;
    let factory = features.try_get::<dyn IGlPlatformSurfaceRenderTargetFactory>();
    surfaces.iter().find_map(|surface| match &factory {
        Some(factory) if factory.can_render_to_surface(gl_context, surface) => {
            Some(factory.create_render_target(gl_context, surface))
        }
        _ => try_get_gl_surface(&**surface).map(|gl_surface| gl_surface.create_gl_render_target(gl_context)),
    })
}

/// Whether a context of OpenGL has a surface among `surfaces` to render to.
#[cfg_attr(not(any(target_os = "linux", windows)), allow(dead_code))]
pub(crate) fn has_gl_surface(gl_context: &Rc<dyn IGlContext>, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> Option<bool> {
    let features: &dyn IOptionalFeatureProvider = &**gl_context;
    let factory = features.try_get::<dyn IGlPlatformSurfaceRenderTargetFactory>();
    surfaces
        .iter()
        .find(|surface| {
            factory.as_ref().is_some_and(|factory| factory.can_render_to_surface(gl_context, surface))
                || try_get_gl_surface(&***surface).is_some()
        })
        .map(|surface| surface.is_ready())
}

/// Copies a texture of the context (the frame of a window, its first row the
/// top of the frame) into the framebuffer `target_framebuffer` of `width` by
/// `height` pixels, whose first row is the top of the window when
/// `y_flipped`, the bottom otherwise; `read_framebuffer` is a framebuffer of
/// the caller's the texture is attached to for the copy.
pub(crate) fn blit_texture(
    gl: &GlInterface,
    texture: i32,
    read_framebuffer: i32,
    target_framebuffer: i32,
    width: i32,
    height: i32,
    y_flipped: bool,
) {
    let (dst_y0, dst_y1) = if y_flipped { (0, height) } else { (height, 0) };
    gl.bind_framebuffer(GL_READ_FRAMEBUFFER, read_framebuffer);
    gl.framebuffer_texture_2d(GL_READ_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, texture, 0);
    gl.bind_framebuffer(GL_DRAW_FRAMEBUFFER, target_framebuffer);
    gl.blit_framebuffer(0, 0, width, height, 0, dst_y0, width, dst_y1, GL_COLOR_BUFFER_BIT, GL_NEAREST);
    gl.bind_framebuffer(GL_FRAMEBUFFER, target_framebuffer);
}

/// The texture of the frame of a window in the context, and the framebuffer
/// it is read from.
struct FrameTexture {
    texture: i32,
    framebuffer: i32,
    size: PixelSize,
}

/// The render target of a surface of OpenGL drawn in the CPU mode (see the
/// module). It keeps the frame of the window, says that it retains it and
/// can be rendered to directly, so that the compositor draws only what
/// changed, and uploads only what changed.
pub struct VelloGlRenderTarget {
    gl_context: Rc<dyn IGlContext>,
    target: RefCell<Option<Rc<dyn IGlPlatformSurfaceRenderTarget>>>,
    rendering_modes: Vec<VelloRenderingMode>,
    frame: RefCell<Option<Rc<SurfaceRenderTarget>>>,
    texture: Rc<RefCell<Option<FrameTexture>>>,
}

impl VelloGlRenderTarget {
    fn new(
        gl_context: Rc<dyn IGlContext>,
        target: Rc<dyn IGlPlatformSurfaceRenderTarget>,
        rendering_modes: &[VelloRenderingMode],
    ) -> Self {
        // The frame is drawn in memory: by the CPU mode when the list has
        // it, which is the mode this target is made for.
        let rendering_modes = match rendering_modes.contains(&VelloRenderingMode::Cpu) {
            true => vec![VelloRenderingMode::Cpu],
            false => rendering_modes.to_vec(),
        };
        Self {
            gl_context,
            target: RefCell::new(Some(target)),
            rendering_modes,
            frame: RefCell::new(None),
            texture: Rc::new(RefCell::new(None)),
        }
    }

    /// The frame of the window for a framebuffer of `size` at `dpi`: the one
    /// of the frame before when it still fits, a new one otherwise.
    fn frame_for(&self, size: PixelSize, dpi: ferroui_base::Vector) -> Rc<SurfaceRenderTarget> {
        let mut frame = self.frame.borrow_mut();
        let fits = frame.as_ref().is_some_and(|frame| frame.pixel_size() == size && frame.dpi() == dpi);
        if !fits {
            if let Some(old) = frame.take() {
                IBitmapImpl::dispose(&*old);
            }
            *frame = Some(Rc::new(SurfaceRenderTarget::new(SurfaceRenderTargetCreateInfo {
                width: size.width,
                height: size.height,
                dpi,
                rendering_modes: self.rendering_modes.clone(),
                use_scaled_drawing: false,
            })));
        }
        frame.clone().unwrap_or_else(|| panic!("The frame of the window was just created"))
    }
}

/// Uploads what changed of the frame into its texture (all of it into a new
/// one) and copies the texture into the framebuffer bound now, which the
/// session presents; its first row is the top of the window when
/// `y_flipped`, the bottom otherwise.
fn present_frame(
    gl: &GlInterface,
    texture: &mut Option<FrameTexture>,
    frame: &SurfaceRenderTarget,
    size: PixelSize,
    y_flipped: bool,
) {
    let target_framebuffer = gl.get_integerv(GL_FRAMEBUFFER_BINDING);
    let changed = frame.take_changed();
    let Some(image) = IDrawableBitmapImpl::image(frame) else {
        return;
    };
    let (width, height) = (size.width.max(1), size.height.max(1));
    let pixels = image.data.data();
    if pixels.len() < width as usize * height as usize * 4 {
        return;
    }

    let fits = texture.as_ref().is_some_and(|texture| texture.size == size);
    if !fits {
        if let Some(old) = texture.take() {
            // SAFETY: the names were made by this context, one each.
            unsafe {
                gl.delete_textures(1, &old.texture);
                gl.delete_framebuffers(1, &old.framebuffer);
            }
        }
        let (mut name, mut framebuffer) = (0, 0);
        // SAFETY: one name each is written.
        unsafe {
            gl.gen_textures(1, &mut name);
            gl.gen_framebuffers(1, &mut framebuffer);
        }
        gl.bind_texture(GL_TEXTURE_2D, name);
        gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
        gl.tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
        // SAFETY: `pixels` holds `height` rows of `width` RGBA pixels.
        unsafe {
            gl.tex_image_2d(
                GL_TEXTURE_2D,
                0,
                GL_RGBA,
                width,
                height,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                pixels.as_ptr().cast::<c_void>(),
            );
        }
        gl.bind_framebuffer(GL_READ_FRAMEBUFFER, framebuffer);
        gl.framebuffer_texture_2d(GL_READ_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, name, 0);
        *texture = Some(FrameTexture { texture: name, framebuffer, size });
    } else if let Some(change) = changed {
        let frame_texture = texture.as_ref().expect("the texture fits");
        let (x0, y0, x1, y1) = match change {
            SurfaceChange::All => (0, 0, width, height),
            SurfaceChange::Rect(rect) => (
                i32::from(rect.x0).min(width),
                i32::from(rect.y0).min(height),
                i32::from(rect.x1).min(width),
                i32::from(rect.y1).min(height),
            ),
        };
        if x1 > x0 && y1 > y0 {
            gl.bind_texture(GL_TEXTURE_2D, frame_texture.texture);
            gl.pixel_storei(GL_UNPACK_ALIGNMENT, 4);
            gl.pixel_storei(GL_UNPACK_ROW_LENGTH, width);
            let offset = (y0 as usize * width as usize + x0 as usize) * 4;
            let _perf = crate::perf::scope(crate::perf::Phase::Upload, ((x1 - x0) * (y1 - y0) * 4) as u64);
            // SAFETY: the rows of the rectangle lie in `pixels`, `width`
            // pixels apart (the row length of the unpacking).
            unsafe {
                gl.tex_sub_image_2d(
                    GL_TEXTURE_2D,
                    0,
                    x0,
                    y0,
                    x1 - x0,
                    y1 - y0,
                    GL_RGBA,
                    GL_UNSIGNED_BYTE,
                    pixels[offset..].as_ptr().cast::<c_void>(),
                );
            }
            gl.pixel_storei(GL_UNPACK_ROW_LENGTH, 0);
        }
    }

    let Some(frame_texture) = texture.as_ref() else {
        return;
    };
    // The rows of the texture go up in the coordinates of OpenGL: its first
    // row, the top of the frame, is copied to the top of a framebuffer that
    // is not turned over.
    blit_texture(gl, frame_texture.texture, frame_texture.framebuffer, target_framebuffer, width, height, y_flipped);
}

impl IRenderTarget for VelloGlRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        // The rotation of a surface mounted in another orientation (a DRM output) is drawn by
        // the compositor (upstream 9b3e9c881b).
        let target = self.target.borrow();
        let rotation = target.as_ref().map(|target| target.properties().required_rotation_for_orientation);
        RenderTargetProperties {
            retains_previous_frame_contents: true,
            is_suitable_for_direct_rendering: true,
            required_rotation_for_orientation: rotation.unwrap_or_default(),
        }
    }

    fn create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        let target = self.target.borrow().clone().unwrap_or_else(|| panic!("VelloGlRenderTarget has been disposed"));
        let frame_start = crate::perf::frame_start();

        // The session makes the context current with the surface, until it
        // is disposed.
        let session = target.begin_draw(scene_info);
        let size = session.size();
        let size = PixelSize::new(size.width.max(1), size.height.max(1));
        let frame = self.frame_for(size, VelloPlatform::default_dpi() * session.scaling());
        let properties = RenderTargetDrawingContextProperties { previous_frame_is_retained: frame.has_content() };

        let (gl_context, texture, drawn) = (self.gl_context.clone(), self.texture.clone(), frame.clone());
        let disposables: Vec<Box<dyn FnOnce()>> = vec![Box::new(move || {
            {
                let _perf = crate::perf::scope(crate::perf::Phase::TextureCopy, size.width as u64 * size.height as u64 * 4);
                let gl = gl_context.gl_interface();
                present_frame(&gl, &mut texture.borrow_mut(), &drawn, size, session.is_y_flipped());
            }
            {
                let _perf = crate::perf::scope(crate::perf::Phase::Present, 0);
                session.dispose();
            }
            crate::perf::frame_end(frame_start, "window", size.width as u32, size.height as u32);
        })];

        (Box::new(frame.create_drawing_context_with(disposables)), properties)
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        match &*self.target.borrow() {
            Some(target) => target.state(),
            None => PlatformRenderTargetState::DISPOSED,
        }
    }

    fn dispose(&self) {
        if let Some(texture) = self.texture.borrow_mut().take() {
            let current = self.gl_context.make_current();
            let gl = self.gl_context.gl_interface();
            // SAFETY: the names were made by this context, one each.
            unsafe {
                gl.delete_textures(1, &texture.texture);
                gl.delete_framebuffers(1, &texture.framebuffer);
            }
            current.dispose();
        }
        if let Some(frame) = self.frame.borrow_mut().take() {
            IBitmapImpl::dispose(&*frame);
        }
        if let Some(target) = self.target.borrow_mut().take() {
            target.dispose();
        }
    }
}
