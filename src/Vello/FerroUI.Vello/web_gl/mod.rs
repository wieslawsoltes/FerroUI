//! The hybrid mode in a web page: the WebGL2 renderer of `vello_gpu` over
//! the WebGL2 context of a canvas.
//!
//! Built with the feature `hybrid-webgl` of the crate, without `wgpu`:
//! `vello_gpu` draws through the bindings of `web-sys`, which the browser
//! platform of the port links on `wasm32-unknown-emscripten`
//! (`docs/porting/vello-backend.md`, section 12, for why this renderer and
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
//!
//! Step 3 of the GPU route (docs/porting/gpu-rendering.md, #623): a scene paints with textures
//! of the context ([`VelloWebGlTexture`], `frame_textures.rs`), and a layer with an effect is
//! drawn on the context: its content rendered and copied into a texture of the pool, passed
//! through a program of `layer_programs.rs` into another, which the scene the layer was pushed
//! in paints with. The shader effects of the library run their GLSL ES 3.0, translated at build
//! time (`shader_programs.rs`).

mod external_objects;
mod frame_textures;
mod layer_programs;
#[cfg(feature = "media3d")]
mod scene3d;
mod shader_programs;

pub use external_objects::{VelloWebGlExternalObjectsFeature, VelloWebGlFrameBitmap};
pub use frame_textures::VelloWebGlTexture;
use frame_textures::TexturePool;
pub(crate) use layer_programs::{LayerProgram, PassInput};
use layer_programs::LayerPrograms;
use shader_programs::ShaderPrograms;

use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::helpers::pixel_format_helper::scene_size;
use crate::scene::image_textures::{release_unused_textures, rgba_bytes};
use crate::scene::VelloWebGlSceneSink;
use crate::vello_options::VelloRenderingMode;
use crate::vello_platform::VelloPlatform;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::effects::ImmutableCustomEffect;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IDrawingContextImpl, IOptionalFeatureProvider, IPlatformGraphicsContext, IRenderTarget, PlatformRenderTargetState,
    RenderTargetDrawingContextProperties, RenderTargetProperties, RenderTargetSceneInfo,
};
use ferroui_opengl::gl_consts::{GL_COLOR_ATTACHMENT0, GL_READ_FRAMEBUFFER, GL_TEXTURE_2D};
use ferroui_opengl::surfaces::{try_get_gl_surface, IGlPlatformSurfaceRenderTarget};
use ferroui_opengl::IGlContext;
use peniko::color::AlphaColor;
use peniko::ImageData;
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use vello_gpu::{RenderSettings, RenderSize, Resources, Scene, TextureId, WebGlRenderer, WebGlTextureBindings};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext, WebGlTexture};

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

/// The texture of an image, the render it was last drawn in and its bytes.
struct ImageTexture {
    texture: WebGlTexture,
    last_used: u64,
    bytes: u64,
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
    /// The textures scenes paint with that were written on the context.
    textures: TexturePool,
    /// The programs of the layers with an effect, compiled when first needed.
    programs: LayerPrograms,
    /// The programs of the shader effects, compiled when first needed.
    shaders: ShaderPrograms,
    /// The GL executor of 3D scenes and its kept outputs, made when a scene is first drawn.
    #[cfg(feature = "media3d")]
    scenes: scene3d::WebGlScenes,
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
    /// Names this GPU among those of the process: the frame bitmaps of its pool say which.
    identity: u64,
    /// The feature that imports the frames of producers, made when first asked for.
    external_objects: RefCell<Option<Rc<VelloWebGlExternalObjectsFeature>>>,
    /// The framebuffer of the OpenGL contracts a frame is read through; 0 until the first frame.
    frame_read_framebuffer: Cell<i32>,
}

/// The identity of the next GPU.
static NEXT_GPU: AtomicU64 = AtomicU64::new(1);

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
            state: RefCell::new(Some(WebGlState {
                renderer,
                resources,
                images: HashMap::new(),
                render_count: 0,
                textures: TexturePool::default(),
                programs: LayerPrograms::default(),
                shaders: ShaderPrograms::default(),
                #[cfg(feature = "media3d")]
                scenes: scene3d::WebGlScenes::default(),
            })),
            identity: NEXT_GPU.fetch_add(1, Ordering::Relaxed),
            external_objects: RefCell::new(None),
            frame_read_framebuffer: Cell::new(0),
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
        let framebuffer = self.frame_read_framebuffer.replace(0);
        if framebuffer != 0 {
            self.gl_context.gl_interface().delete_framebuffer(framebuffer);
        }
        if let Some(mut state) = self.state.borrow_mut().take() {
            for image in state.images.values() {
                self.gl.delete_texture(Some(&image.texture));
            }
            state.textures.dispose(&self.gl);
            state.programs.dispose(&self.gl);
            state.shaders.dispose(&self.gl);
            #[cfg(feature = "media3d")]
            state.scenes.dispose(&self.gl_context);
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
    /// identity of their pixels, and `textures` the textures of the context it paints with.
    pub(crate) fn render(
        &self,
        scene: &Scene,
        images: &HashMap<u64, ImageData>,
        textures: &[Rc<VelloWebGlTexture>],
    ) -> Result<(), String> {
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
                state.images.insert(*id, ImageTexture { texture, last_used: 0, bytes: rgba_bytes(image.width, image.height) });
            }
            let texture = state.images.get_mut(id).expect("the texture was just inserted");
            texture.last_used = render_count;
            bindings.insert(TextureId(*id), texture.texture.clone());
        }
        for texture in textures {
            bindings.insert(texture.id, texture.texture.clone());
        }

        let result = state.renderer.render(
            scene,
            &mut state.resources,
            &RenderSize { width: scene.width(), height: scene.height() },
            &bindings,
            AlphaColor::TRANSPARENT,
        );

        let gl = &self.gl;
        release_unused_textures(
            &mut state.images,
            render_count,
            |image| (image.last_used, image.bytes),
            |image| gl.delete_texture(Some(&image.texture)),
        );

        result.map_err(|error| error.to_string())
    }

    /// A texture of the pool for an area of `width` by `height` pixels (its size is a size of the
    /// pool, at least the area).
    pub(crate) fn acquire_texture(&self, width: u32, height: u32) -> Result<Rc<VelloWebGlTexture>, String> {
        let (width, height) = frame_textures::pooled_size(width, height, self.max_texture_size);
        let mut state = self.state.borrow_mut();
        let state = state.as_mut().ok_or("the renderer of the canvas has been released")?;
        state.textures.acquire(&self.gl, width, height)
    }

    /// Renders the scene of the content of a layer, which has the size of the drawing buffer,
    /// into the drawing buffer, and copies the pixels of `area` (`x`, `y`, `width`, `height`, from
    /// the top left of the canvas) into a texture of the pool, its first row the top one of the
    /// area. The drawing buffer holds nothing of the frame: the scene of the frame is rendered
    /// after its layers and clears it.
    pub(crate) fn render_to_texture(
        &self,
        scene: &Scene,
        images: &HashMap<u64, ImageData>,
        textures: &[Rc<VelloWebGlTexture>],
        area: (u32, u32, u32, u32),
    ) -> Result<Rc<VelloWebGlTexture>, String> {
        self.render(scene, images, textures)?;
        let (x, y, width, height) = area;
        let copied = self.acquire_texture(width, height)?;
        let gl = &self.gl;
        // The rows of the drawing buffer count from its bottom. The copy keeps them in that order,
        // so the first row of the texture is the bottom one of the area; the passes that read it
        // turn it upright (`layer_programs.rs`).
        let bottom = gl.drawing_buffer_height() - (y + height) as i32;
        gl.bind_framebuffer(WebGl2RenderingContext::READ_FRAMEBUFFER, None);
        gl.active_texture(WebGl2RenderingContext::TEXTURE0);
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, Some(&copied.texture));
        gl.copy_tex_sub_image_2d(
            WebGl2RenderingContext::TEXTURE_2D,
            0,
            0,
            0,
            x as i32,
            bottom,
            width as i32,
            height as i32,
        );
        gl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, None);
        Ok(copied)
    }

    /// Whether a layer that `program` draws can be drawn on the context: the program compiled
    /// (now, the first time).
    pub(crate) fn prepare(&self, program: LayerProgram) -> bool {
        match self.state.borrow_mut().as_mut() {
            Some(state) => state.programs.prepare(&self.gl, program),
            None => false,
        }
    }

    /// The pass of `program` on the context: its `uniforms` (`effect_uniforms.rs`) over the `area`
    /// (width, height) of the content of a layer in `inputs` (textures of the pool, each with
    /// whether its first row is the bottom one of the area, as
    /// [`render_to_texture`](Self::render_to_texture) leaves it), into a texture of the pool,
    /// upright. `None` when the program is not there (it did not compile: the layer is drawn on
    /// the processor) or the inputs do not fit it.
    pub(crate) fn layer_texture(
        &self,
        program: LayerProgram,
        inputs: &[PassInput<'_>],
        area: (u32, u32),
        uniforms: &[f32],
    ) -> Result<Option<Rc<VelloWebGlTexture>>, String> {
        let mut state = self.state.borrow_mut();
        let state = state.as_mut().ok_or("the renderer of the canvas has been released")?;
        let (width, height) = frame_textures::pooled_size(area.0, area.1, self.max_texture_size);
        let target = state.textures.acquire(&self.gl, width, height)?;
        let drawn = state.programs.pass(&self.gl, program, inputs, &target, area, uniforms);
        Ok(drawn.then_some(target))
    }

    /// The texture of a 3D scene drawn at `size` pixels on the context, with the scene at its top
    /// left (`scene3d.rs`). `None` when the context cannot draw it: the CPU reference draws it.
    #[cfg(feature = "media3d")]
    pub(crate) fn scene_texture(
        &self,
        scene: &Arc<ferroui_base::rendering::scene3d::Scene3DSnapshot>,
        size: (u32, u32),
        scale: f64,
        antialiased: bool,
        origin: (i32, i32),
    ) -> Option<(Rc<VelloWebGlTexture>, (u32, u32))> {
        if self.is_context_lost() {
            return None;
        }
        let mut state = self.state.borrow_mut();
        let WebGlState { scenes, textures, .. } = state.as_mut()?;
        let limit = self.max_texture_size;
        let gpu = self.identity;
        scenes.scene_texture(&self.gl, &self.gl_context, textures, limit, gpu, scene, size, scale, antialiased, origin)
    }

    /// What names this GPU: the frame bitmaps of its pool carry it.
    pub(crate) fn identity(&self) -> u64 {
        self.identity
    }

    /// The feature that imports the frames of producers (#575, `external_objects.rs`); `None` for
    /// a context that takes none.
    pub(crate) fn external_objects(self: &Rc<Self>) -> Option<Rc<VelloWebGlExternalObjectsFeature>> {
        let mut feature = self.external_objects.borrow_mut();
        if feature.is_none() {
            let weak = Rc::downgrade(self);
            *feature = VelloWebGlExternalObjectsFeature::new(weak, self.gl_context.clone()).map(Rc::new);
        }
        feature.clone()
    }

    /// The texture `name` of the context (a name of the OpenGL contracts, RGBA8) copied on the GPU
    /// into a texture of the pool, its first row the top one of the frame; `bottom_up` when the
    /// first row of `name` is the bottom one.
    pub(crate) fn copy_frame_texture(
        &self,
        name: u32,
        (width, height): (u32, u32),
        bottom_up: bool,
    ) -> Result<Rc<VelloWebGlTexture>, String> {
        external_objects::release_retired();
        let copy = self.acquire_texture(width, height)?;
        let current = self.gl_context.ensure_current();
        let interface = self.gl_context.gl_interface();
        if self.frame_read_framebuffer.get() == 0 {
            self.frame_read_framebuffer.set(interface.gen_framebuffer());
        }
        let gl = &self.gl;
        let scissor = gl.is_enabled(WebGl2RenderingContext::SCISSOR_TEST);
        gl.disable(WebGl2RenderingContext::SCISSOR_TEST);
        interface.bind_framebuffer(GL_READ_FRAMEBUFFER, self.frame_read_framebuffer.get());
        interface.framebuffer_texture_2d(GL_READ_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, name as i32, 0);
        gl.bind_framebuffer(WebGl2RenderingContext::DRAW_FRAMEBUFFER, Some(&copy.framebuffer));
        let (width, height) = (width as i32, height as i32);
        let (top, bottom) = if bottom_up { (height, 0) } else { (0, height) };
        gl.blit_framebuffer(
            0,
            0,
            width,
            height,
            0,
            top,
            width,
            bottom,
            WebGl2RenderingContext::COLOR_BUFFER_BIT,
            WebGl2RenderingContext::NEAREST,
        );
        interface.framebuffer_texture_2d(GL_READ_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, 0, 0);
        gl.bind_framebuffer(WebGl2RenderingContext::READ_FRAMEBUFFER, None);
        gl.bind_framebuffer(WebGl2RenderingContext::DRAW_FRAMEBUFFER, None);
        if scissor {
            gl.enable(WebGl2RenderingContext::SCISSOR_TEST);
        }
        current.dispose();
        Ok(copy)
    }

    /// Whether the shader effect `effect` is drawn on the context: it is one of the library's and
    /// its program compiled (now, the first time).
    pub(crate) fn prepare_shader(&self, effect: &ImmutableCustomEffect) -> bool {
        match self.state.borrow_mut().as_mut() {
            Some(state) => state.shaders.prepare(&self.gl, effect),
            None => false,
        }
    }

    /// The content `copied` as [`render_to_texture`](Self::render_to_texture) leaves it, its first
    /// row the bottom one of `area` (width, height), in a texture of the pool with its first row
    /// the top one: what the program of a shader effect reads by texel.
    pub(crate) fn upright_texture(
        &self,
        copied: &Rc<VelloWebGlTexture>,
        (width, height): (u32, u32),
    ) -> Result<Rc<VelloWebGlTexture>, String> {
        let upright = self.acquire_texture(width, height)?;
        let gl = &self.gl;
        let scissor = gl.is_enabled(WebGl2RenderingContext::SCISSOR_TEST);
        gl.disable(WebGl2RenderingContext::SCISSOR_TEST);
        gl.bind_framebuffer(WebGl2RenderingContext::READ_FRAMEBUFFER, Some(&copied.framebuffer));
        gl.bind_framebuffer(WebGl2RenderingContext::DRAW_FRAMEBUFFER, Some(&upright.framebuffer));
        let (width, height) = (width as i32, height as i32);
        gl.blit_framebuffer(
            0,
            0,
            width,
            height,
            0,
            height,
            width,
            0,
            WebGl2RenderingContext::COLOR_BUFFER_BIT,
            WebGl2RenderingContext::NEAREST,
        );
        gl.bind_framebuffer(WebGl2RenderingContext::READ_FRAMEBUFFER, None);
        gl.bind_framebuffer(WebGl2RenderingContext::DRAW_FRAMEBUFFER, None);
        if scissor {
            gl.enable(WebGl2RenderingContext::SCISSOR_TEST);
        }
        Ok(upright)
    }

    /// The pass of the shader effect `effect` over `content` (upright, holding `area` pixels whose
    /// top left is `origin` of the area of the effect, of `size` pixels at `scale`) into a texture
    /// of the pool. `None` when its program is not there or its uniforms do not match it.
    pub(crate) fn shader_effect_texture(
        &self,
        effect: &ImmutableCustomEffect,
        content: &Rc<VelloWebGlTexture>,
        area: (u32, u32),
        origin: [f32; 2],
        size: [f32; 2],
        scale: f32,
    ) -> Result<Option<Rc<VelloWebGlTexture>>, String> {
        let mut state = self.state.borrow_mut();
        let state = state.as_mut().ok_or("the renderer of the canvas has been released")?;
        let (width, height) = frame_textures::pooled_size(area.0, area.1, self.max_texture_size);
        let target = state.textures.acquire(&self.gl, width, height)?;
        let drawn = state.shaders.pass(&self.gl, effect, content, &target, area, origin, size, scale);
        Ok(drawn.then_some(target))
    }

    /// Lets go of the textures of the pool that the frames no longer use, beyond a few of each
    /// size.
    pub(crate) fn trim_textures(&self) {
        if let Some(state) = self.state.borrow_mut().as_mut() {
            state.textures.trim(&self.gl, 4);
        }
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
