//! 3D scenes on the WebGL2 context of the canvas (#608 D8, #623, step 3 of the GPU route of
//! docs/porting/gpu-rendering.md): the GL executor of `ferroui-opengl` draws the scene on the
//! context, and the result is copied into a texture of the pool that the scene of Vello paints with.
//!
//! The executor speaks to the context through the OpenGL contracts (in a web page, the WebGL
//! emulation of the module) and the renderer of `vello_gpu` through `web-sys`: one context, two
//! sets of names. An output of the executor is therefore read through a framebuffer of the
//! contracts and copied with `copyTexSubImage2D` into a texture of the pool. Nothing is read back.
//!
//! - **Kept outputs.** Up to eight per context. A scene whose snapshot, generations, size, scale,
//!   edge mode and brush textures are those of a kept output paints its copy again, with no 3D pass.
//! - **A frame being recorded.** A render takes a new texture of the pool for its copy, so a scene
//!   of this frame that already paints the old copy keeps it; the pool has it back afterwards.
//! - **State.** The executor changes the state of the context and does not restore it. What the
//!   renderer set once when it was created (blending) is put back after each render.

use super::frame_textures::{pooled_size, TexturePool};
use super::VelloWebGlTexture;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::rendering::scene3d::Scene3DSnapshot;
use ferroui_opengl::gl_consts::{GL_COLOR_ATTACHMENT0, GL_READ_FRAMEBUFFER, GL_TEXTURE_2D};
use ferroui_opengl::scene3d::{
    Scene3DGlError, Scene3DGlExecutor, Scene3DGlTarget, Scene3DGlTexture, Scene3DGlViewport,
};
use ferroui_opengl::IGlContext;
use ferroui_render3d::brush::scene_texture_inputs;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use web_sys::WebGl2RenderingContext as Gl;

/// The outputs a context keeps: the viewports of a window or two.
const CAPACITY: usize = 8;

/// A kept output.
struct Slot {
    scene: Weak<Scene3DSnapshot>,
    generations: (u64, u64, u64),
    size: (u32, u32),
    scale_bits: u64,
    antialiased: bool,
    /// The textures of the image brushes of the scene.
    inputs: u64,
    /// The device pixel of the top left of the viewport.
    origin: (i32, i32),
    texture: Scene3DGlTexture,
    viewport: Scene3DGlViewport,
    /// The copy of the output the scenes of Vello paint with; `None` until the first render.
    copy: Option<Rc<VelloWebGlTexture>>,
    last_used: u64,
}

/// The executor of a context and its kept outputs.
#[derive(Default)]
pub(super) struct WebGlScenes {
    /// `None` until a scene is first drawn; an error for a context that draws the CPU reference.
    executor: Option<Result<Scene3DGlExecutor, Scene3DGlError>>,
    /// The framebuffer of the contracts an output is read through.
    read_framebuffer: i32,
    slots: Vec<Slot>,
    clock: u64,
}

static FAILURE_REPORTED: AtomicBool = AtomicBool::new(false);

/// Reports once that the context draws its 3D scenes with the CPU reference, and why.
fn report_failure(error: &Scene3DGlError) {
    if FAILURE_REPORTED.swap(true, Ordering::Relaxed) {
        return;
    }
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
        let reason = error.reason.as_str();
        logger.log_with_values(
            None,
            "The WebGL2 route of the Vello backend draws 3D scenes (Viewport3D) with the CPU reference: \
             {Reason} (docs/porting/media3d.md, section 3.20).",
            &[&reason],
        );
    }
}

/// Puts back what the renderer of `vello_gpu` set once and the executor changed.
fn restore_state(gl: &Gl) {
    gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
    gl.bind_vertex_array(None);
    gl.use_program(None);
    gl.bind_buffer(Gl::ARRAY_BUFFER, None);
    gl.bind_buffer(Gl::UNIFORM_BUFFER, None);
    for unit in 0..2 {
        gl.active_texture(Gl::TEXTURE0 + unit);
        gl.bind_texture(Gl::TEXTURE_2D, None);
    }
    gl.active_texture(Gl::TEXTURE0);
    gl.disable(Gl::SCISSOR_TEST);
    gl.disable(Gl::DEPTH_TEST);
    gl.disable(Gl::CULL_FACE);
    gl.enable(Gl::DITHER);
    gl.front_face(Gl::CCW);
    gl.depth_mask(true);
    gl.color_mask(true, true, true, true);
    gl.enable(Gl::BLEND);
    gl.blend_equation(Gl::FUNC_ADD);
    gl.blend_func(Gl::ONE, Gl::ONE_MINUS_SRC_ALPHA);
    gl.viewport(0, 0, gl.drawing_buffer_width(), gl.drawing_buffer_height());
}

impl WebGlScenes {
    /// The texture of `scene` drawn at `size` pixels, with the pixels of the scene at its top
    /// left. `None` when the context cannot draw it: the caller draws the CPU reference.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn scene_texture(
        &mut self,
        gl: &Gl,
        context: &Rc<dyn IGlContext>,
        pool: &mut TexturePool,
        max_texture_size: u32,
        scene: &Arc<Scene3DSnapshot>,
        size: (u32, u32),
        scale: f64,
        antialiased: bool,
        origin: (i32, i32),
    ) -> Option<(Rc<VelloWebGlTexture>, (u32, u32))> {
        if size.0 > max_texture_size || size.1 > max_texture_size {
            return None;
        }
        let interface = context.gl_interface();
        if self.executor.is_none() {
            let executor = Scene3DGlExecutor::new(interface.clone(), context.version());
            if let Err(error) = &executor {
                report_failure(error);
            }
            self.executor = Some(executor);
            self.read_framebuffer = interface.gen_framebuffer();
        }
        let Some(Ok(executor)) = &mut self.executor else { return None };
        self.clock += 1;
        let clock = self.clock;
        let scale_bits = scale.to_bits();
        let generations = scene.generations();
        let inputs = scene_texture_inputs(scene);

        // 1. A kept output of this scene.
        let same_scene = |slot: &Slot| std::ptr::eq(slot.scene.as_ptr(), Arc::as_ptr(scene));
        if let Some(slot) = self.slots.iter_mut().find(|slot| {
            let slot: &Slot = slot;
            same_scene(slot)
                && slot.generations == generations
                && slot.size == size
                && slot.scale_bits == scale_bits
                && slot.antialiased == antialiased
                && slot.inputs == inputs
                && slot.copy.is_some()
        }) {
            slot.last_used = clock;
            return slot.copy.clone().map(|copy| (copy, size));
        }

        // 2. The output to draw into: the same viewport, the output of a scene that is gone, a
        //    new one, or the least recently used.
        let gone = |slot: &Slot| slot.scene.strong_count() == 0;
        let same_viewport = |slot: &Slot| slot.origin == origin && slot.size == size;
        let chosen = self
            .slots
            .iter()
            .position(|slot| same_viewport(slot) && (gone(slot) || same_scene(slot)))
            .or_else(|| self.slots.iter().position(same_viewport))
            .or_else(|| self.slots.iter().position(gone));
        let least_used = || {
            self.slots.iter().enumerate().min_by_key(|(_, slot)| slot.last_used).map(|(index, _)| index)
        };
        let texture_size = pooled_size(size.0, size.1, max_texture_size);
        let index = match chosen.or_else(|| least_used().filter(|_| self.slots.len() >= CAPACITY)) {
            Some(index) => index,
            None => {
                let texture = executor.create_output_texture(texture_size.0, texture_size.1)?;
                self.slots.push(Slot {
                    scene: Weak::new(),
                    generations: (0, 0, 0),
                    size,
                    scale_bits: 0,
                    antialiased: false,
                    inputs: 0,
                    origin,
                    texture,
                    viewport: Scene3DGlViewport::new(),
                    copy: None,
                    last_used: clock,
                });
                self.slots.len() - 1
            }
        };
        let slot = &mut self.slots[index];
        if (slot.texture.width(), slot.texture.height()) != texture_size {
            let texture = executor.create_output_texture(texture_size.0, texture_size.1)?;
            executor.delete_output_texture(std::mem::replace(&mut slot.texture, texture));
        }
        let copy = match pool.acquire(gl, texture_size.0, texture_size.1) {
            Ok(copy) => copy,
            Err(error) => {
                super::log_render_failure(&error);
                return None;
            }
        };

        // 3. The render, and the copy into the texture of the pool.
        let target = Scene3DGlTarget { texture: &slot.texture, size, antialiased };
        let rendered = executor.render(&mut slot.viewport, scene, &target);
        if rendered.is_ok() {
            interface.bind_framebuffer(GL_READ_FRAMEBUFFER, self.read_framebuffer);
            interface.framebuffer_texture_2d(
                GL_READ_FRAMEBUFFER,
                GL_COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                slot.texture.id(),
                0,
            );
            gl.active_texture(Gl::TEXTURE0);
            gl.bind_texture(Gl::TEXTURE_2D, Some(&copy.texture));
            // The first row of the output is the top of the scene, as it is in the copy.
            gl.copy_tex_sub_image_2d(Gl::TEXTURE_2D, 0, 0, 0, 0, 0, size.0 as i32, size.1 as i32);
            gl.bind_texture(Gl::TEXTURE_2D, None);
            interface.framebuffer_texture_2d(GL_READ_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, 0, 0);
            interface.bind_framebuffer(GL_READ_FRAMEBUFFER, 0);
        }
        restore_state(gl);
        match rendered {
            Ok(_) => {
                slot.scene = Arc::downgrade(scene);
                slot.generations = generations;
                slot.size = size;
                slot.scale_bits = scale_bits;
                slot.antialiased = antialiased;
                slot.inputs = inputs;
                slot.origin = origin;
                slot.last_used = clock;
                slot.copy = Some(copy.clone());
                Some((copy, size))
            }
            Err(error) => {
                // The context draws its 3D scenes with the CPU reference from now on.
                report_failure(&error);
                for mut slot in self.slots.drain(..) {
                    slot.viewport.dispose(executor);
                    executor.delete_output_texture(slot.texture);
                }
                executor.dispose();
                self.executor = Some(Err(error));
                None
            }
        }
    }

    /// Deletes the outputs and the executor: the context is current and being released.
    pub(super) fn dispose(&mut self, context: &Rc<dyn IGlContext>) {
        if let Some(Ok(executor)) = &mut self.executor {
            for mut slot in self.slots.drain(..) {
                slot.viewport.dispose(executor);
                executor.delete_output_texture(slot.texture);
            }
            executor.dispose();
        }
        if self.read_framebuffer != 0 {
            context.gl_interface().delete_framebuffer(self.read_framebuffer);
            self.read_framebuffer = 0;
        }
        self.slots.clear();
        self.executor = None;
    }
}
