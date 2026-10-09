//! The hybrid mode in a web page: the WebGL2 renderer of `vello_gpu` over
//! the WebGL2 context of a canvas.
//!
//! Built with the feature `hybrid-webgl` of the crate, without `wgpu`:
//! `vello_gpu` draws through the bindings of `web-sys`, which the browser
//! platform of the port links on `wasm32-unknown-emscripten`
//! (`docs/porting/vello-backend.md`, section 11, for why this renderer and
//! not `wgpu` over WebGL).
//!
//! What corresponds to what in the Skia backend, which draws to the same
//! canvas with Ganesh over the OpenGL contracts (`gpu/open_gl` there):
//!
//! | Skia | here |
//! |---|---|
//! | `GlSkiaGpu` over an `IGlContext` | [`VelloWebGlGpu`] over an `IGlContext` that also hands out its canvas ([`IWebGlCanvasFeature`]) |
//! | `GlRenderTarget` over an `IGlPlatformSurfaceRenderTarget` | [`VelloWebGlRenderTarget`] over the same |
//! | a session whose surface wraps the framebuffer of the canvas | a scene ([`VelloWebGlSceneSink`](crate::scene::VelloWebGlSceneSink)) rendered into the drawing buffer of the canvas when the drawing context is disposed |
//!
//! Everything here belongs to the thread that draws to the canvas: the
//! thread of the page, or the render worker the canvas was transferred to.
//! Nothing is `Send`.

use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::helpers::pixel_format_helper::scene_size;
use crate::scene::VelloWebGlSceneSink;
use crate::vello_options::VelloRenderingMode;
use crate::vello_platform::VelloPlatform;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IDrawingContextImpl, IOptionalFeatureProvider, IPlatformGraphicsContext, IRenderTarget, PlatformRenderTargetState,
    RenderTargetDrawingContextProperties, RenderTargetProperties, RenderTargetSceneInfo,
};
use ferroui_opengl::surfaces::{try_get_gl_surface, IGlPlatformSurfaceRenderTarget};
use ferroui_opengl::IGlContext;
use peniko::color::AlphaColor;
use peniko::ImageData;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use vello_gpu::{RenderSettings, RenderSize, Resources, Scene, TextureId, WebGlRenderer, WebGlTextureBindings};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext, WebGlTexture};

/// The number of renders the texture of an image that is no longer drawn is
/// kept for: an image that is drawn every frame is uploaded once, and one
/// that left the scene is released soon after. The number of the hybrid
/// sink over `wgpu`.
const IMAGE_TEXTURE_LIFETIME: u64 = 16;

/// A feature of a platform graphics context that is the WebGL2 context of a
/// canvas: the canvas.
///
/// The OpenGL contracts hand out the entry points of a context as the
/// functions of a C library (in a web page: the WebGL emulation of the
/// module). The renderer of `vello_gpu` calls the context of the page
/// itself, which it gets from its canvas; a context that is one hands the
/// canvas out here, beside its `IGlContext`.
pub trait IWebGlCanvasFeature {
    /// The canvas the context was created for: an `HTMLCanvasElement`, or
    /// an `OffscreenCanvas` on the worker it was transferred to. An object
    /// of the thread that asks.
    fn canvas(&self) -> JsValue;
}

/// The texture of an image and the render it was last drawn in.
struct ImageTexture {
    texture: WebGlTexture,
    last_used: u64,
}

/// What the renderer keeps on the context between frames.
struct WebGlState {
    renderer: WebGlRenderer,
    /// The caches of the renderer: the glyphs and the image atlas.
    resources: Resources,
    /// The textures of the images of the last frames, by the identity of
    /// their pixels.
    images: HashMap<u64, ImageTexture>,
    render_count: u64,
}

/// The WebGL2 context of a canvas as the Vello backend draws with it: what
/// `GlSkiaGpu` is to the Skia backend.
pub struct VelloWebGlGpu {
    graphics_context: Rc<dyn IPlatformGraphicsContext>,
    gl_context: Rc<dyn IGlContext>,
    gl: WebGl2RenderingContext,
    max_texture_size: u32,
    /// `None` once the GPU is disposed: the context draws nothing more.
    state: RefCell<Option<WebGlState>>,
}

impl VelloWebGlGpu {
    /// The GPU of a graphics context that is the WebGL2 context of a
    /// canvas; `None` for a context that is not one (it has no
    /// [`IWebGlCanvasFeature`] or no `IGlContext`).
    ///
    /// Compiles and links the shaders of the renderer before it returns.
    ///
    /// # Errors
    /// The reason the renderer gave for not taking the context: the canvas
    /// has no WebGL2 context, the context was created with anti-aliasing
    /// or with a depth buffer of less than 24 bits, a shader did not
    /// compile.
    pub fn try_new(graphics_context: &Rc<dyn IPlatformGraphicsContext>) -> Option<Result<Rc<Self>, String>> {
        let features: &dyn IOptionalFeatureProvider = &**graphics_context;
        let canvas = features.try_get::<dyn IWebGlCanvasFeature>()?.canvas();
        let gl_context = features.try_get::<dyn IGlContext>()?;

        // The renderer asks the canvas for its WebGL2 context, and a canvas
        // that has one returns it whatever the options of the call are: the
        // context the platform created, with the attributes it created it
        // with. `getContext` is a member of both kinds of canvas with the
        // same arguments, and the binding calls it by name.
        let canvas: &HtmlCanvasElement = canvas.unchecked_ref();
        let (renderer, resources) = match WebGlRenderer::new_with(canvas, RenderSettings::default(), true) {
            Ok(created) => created,
            Err(error) => return Some(Err(error.to_string())),
        };

        let gl = renderer.gl_context().clone();
        let max_texture_size = gl
            .get_parameter(WebGl2RenderingContext::MAX_TEXTURE_SIZE)
            .ok()
            .and_then(|size| size.as_f64())
            .map_or(0, |size| size as u32);

        Some(Ok(Rc::new(Self {
            graphics_context: graphics_context.clone(),
            gl_context,
            gl,
            max_texture_size,
            state: RefCell::new(Some(WebGlState { renderer, resources, images: HashMap::new(), render_count: 0 })),
        })))
    }

    /// The graphics context of the platform this GPU draws with.
    pub fn platform_graphics_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        self.graphics_context.clone()
    }

    /// Whether the browser took the context away. A lost context draws
    /// nothing; frames are left out while it is lost.
    pub fn is_context_lost(&self) -> bool {
        self.gl.is_context_lost()
    }

    /// The render target of the first of the surfaces that is the canvas of
    /// a WebGL context.
    pub fn try_create_render_target(
        self: &Rc<Self>,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
        rendering_modes: &[VelloRenderingMode],
    ) -> Option<Rc<dyn IRenderTarget>> {
        for surface in surfaces {
            if let Some(gl_surface) = try_get_gl_surface(&**surface) {
                let target = gl_surface.create_gl_render_target(&self.gl_context);
                return Some(Rc::new(VelloWebGlRenderTarget {
                    gpu: self.clone(),
                    target: RefCell::new(Some(target)),
                    rendering_modes: rendering_modes.to_vec(),
                }));
            }
        }

        None
    }

    /// Whether a render target can be created for one of the surfaces now.
    pub fn is_ready_to_create_render_target(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> bool {
        for surface in surfaces {
            if try_get_gl_surface(&**surface).is_some() {
                return surface.is_ready();
            }
        }

        false
    }

    /// Releases what the renderer keeps on the context: its programs and
    /// textures, and the textures of images. The caller has made the
    /// context current, as the compositor does before it releases a backend
    /// context.
    pub fn dispose(&self) {
        if let Some(state) = self.state.borrow_mut().take() {
            for image in state.images.values() {
                self.gl.delete_texture(Some(&image.texture));
            }
        }
    }

    /// Runs `with` on the caches of the renderer, where the glyphs of a
    /// scene are prepared. Does nothing once the GPU is disposed.
    pub(crate) fn with_resources(&self, with: impl FnOnce(&mut Resources)) {
        if let Some(state) = self.state.borrow_mut().as_mut() {
            with(&mut state.resources);
        }
    }

    /// Renders a scene into the drawing buffer of the canvas, replacing
    /// what it held. `images` are the images the scene paints with, by the
    /// identity of their pixels.
    pub(crate) fn render(&self, scene: &Scene, images: &HashMap<u64, ImageData>) -> Result<(), String> {
        if self.is_context_lost() {
            return Err("the WebGL context of the canvas is lost".to_string());
        }

        let mut state = self.state.borrow_mut();
        let Some(state) = state.as_mut() else {
            return Err("the renderer of the canvas has been released".to_string());
        };
        state.render_count += 1;
        let render_count = state.render_count;

        let mut bindings = WebGlTextureBindings::new();
        for (id, image) in images {
            if !state.images.contains_key(id) {
                let texture = self.create_image_texture(image)?;
                state.images.insert(*id, ImageTexture { texture, last_used: 0 });
            }
            let texture = state.images.get_mut(id).expect("the texture was just inserted");
            texture.last_used = render_count;
            bindings.insert(TextureId(*id), texture.texture.clone());
        }

        let result = state.renderer.render(
            scene,
            &mut state.resources,
            &RenderSize { width: scene.width(), height: scene.height() },
            &bindings,
            AlphaColor::TRANSPARENT,
        );

        let gl = &self.gl;
        state.images.retain(|_, image| {
            let keep = render_count - image.last_used < IMAGE_TEXTURE_LIFETIME;
            if !keep {
                gl.delete_texture(Some(&image.texture));
            }
            keep
        });

        result.map_err(|error| error.to_string())
    }

    /// A texture of the context with the pixels of an image, premultiplied,
    /// as the renderer samples a bound texture: by texel, level 0 alone.
    fn create_image_texture(&self, image: &ImageData) -> Result<WebGlTexture, String> {
        if image.width == 0 || image.height == 0 || image.width.max(image.height) > self.max_texture_size {
            return Err(format!(
                "an image of {}x{} pixels does not fit a texture of the context (at most {} in a direction)",
                image.width, image.height, self.max_texture_size
            ));
        }

        let gl = &self.gl;
        let texture = gl.create_texture().ok_or("the context did not create a texture")?;
        let pixels = premultiplied_rgba(image);

        gl.active_texture(WebGl2RenderingContext::TEXTURE0);
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&texture));
        for (name, value) in [
            (WebGl2RenderingContext::TEXTURE_MIN_FILTER, WebGl2RenderingContext::NEAREST),
            (WebGl2RenderingContext::TEXTURE_MAG_FILTER, WebGl2RenderingContext::NEAREST),
            (WebGl2RenderingContext::TEXTURE_WRAP_S, WebGl2RenderingContext::CLAMP_TO_EDGE),
            (WebGl2RenderingContext::TEXTURE_WRAP_T, WebGl2RenderingContext::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, name, value as i32);
        }
        let uploaded = gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            WebGl2RenderingContext::TEXTURE_2D,
            0,
            WebGl2RenderingContext::RGBA8 as i32,
            image.width as i32,
            image.height as i32,
            0,
            WebGl2RenderingContext::RGBA,
            WebGl2RenderingContext::UNSIGNED_BYTE,
            Some(&pixels),
        );
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, None);

        match uploaded {
            Ok(()) => Ok(texture),
            Err(error) => {
                gl.delete_texture(Some(&texture));
                Err(format!("the pixels of an image were not taken by the context: {error:?}"))
            }
        }
    }
}

/// The pixels of an image as premultiplied RGBA, the form the renderer
/// samples. (The function of the same name of the sinks over `wgpu`, which
/// are not compiled where this one is.)
fn premultiplied_rgba(image: &ImageData) -> Cow<'_, [u8]> {
    let data = image.data.data();
    let swap = matches!(image.format, peniko::ImageFormat::Bgra8);
    let premultiply = matches!(image.alpha_type, peniko::ImageAlphaType::Alpha);

    if !swap && !premultiply {
        return Cow::Borrowed(data);
    }

    let mut rgba = data.to_vec();
    for pixel in rgba.chunks_exact_mut(4) {
        if swap {
            pixel.swap(0, 2);
        }
        if premultiply {
            let alpha = u32::from(pixel[3]);
            if alpha != 255 {
                for channel in &mut pixel[..3] {
                    *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
                }
            }
        }
    }
    Cow::Owned(rgba)
}

/// Logs a scene that the renderer could not draw.
pub(crate) fn log_render_failure(error: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
        logger.log_with_values(
            None,
            "The hybrid mode of the Vello backend could not render a scene to its canvas: {Error}",
            &[&error],
        );
    }
}

/// The render target of the canvas of a WebGL2 context: every frame is a
/// session of the platform, during which a scene is recorded and then
/// rendered into the drawing buffer of the canvas.
pub struct VelloWebGlRenderTarget {
    gpu: Rc<VelloWebGlGpu>,
    target: RefCell<Option<Rc<dyn IGlPlatformSurfaceRenderTarget>>>,
    rendering_modes: Vec<VelloRenderingMode>,
}

impl IRenderTarget for VelloWebGlRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        // The scene is recorded for the canvas itself: the compositor draws
        // the visuals into this target, not into a layer that would then be
        // drawn as one image (a layer of this backend is pixels in memory,
        // drawn by the CPU mode: with it the GPU would draw one textured
        // rectangle a frame). The drawing buffer holds nothing of the frame
        // before, which the properties of each drawing context say, so
        // every frame is drawn whole.
        RenderTargetProperties { retains_previous_frame_contents: true, is_suitable_for_direct_rendering: true }
    }

    fn create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        let target =
            self.target.borrow().clone().unwrap_or_else(|| panic!("VelloWebGlRenderTarget has been disposed"));

        // The session gives the canvas the size the view has at this
        // moment; the scene is made for that size.
        let session = target.begin_draw(scene_info);
        let scaling = session.scaling();

        if session.is_y_flipped() {
            session.dispose();
            panic!("The Vello backend does not draw to an OpenGL surface whose origin is its bottom-left corner");
        }

        let (width, height) = scene_size(session.size());
        let sink = VelloWebGlSceneSink::new(self.gpu.clone(), width, height);

        let create_info = CreateInfo {
            sink: Box::new(sink),
            backdrop: None,
            on_finished: Box::new(|sink| {
                // A scene that was not drawn leaves the canvas as it is
                // (a lost context, an intermediate texture that could not
                // be had): the frame is left out, and logged.
                if let Err(error) = sink.render_to_canvas() {
                    log_render_failure(&error);
                }
            }),
            scale_drawing_to_dpi: false,
            dpi: VelloPlatform::default_dpi() * scaling,
            rendering_modes: self.rendering_modes.clone(),
        };

        // Disposing the session ends the frame: the browser presents the
        // drawing buffer when the thread returns to it.
        let context = DrawingContextImpl::new(create_info, vec![Box::new(move || session.dispose())]);

        (Box::new(context), RenderTargetDrawingContextProperties { previous_frame_is_retained: false })
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
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
