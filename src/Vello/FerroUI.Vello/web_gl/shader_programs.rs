//! The runtime shader effects of #455 on the WebGL2 context (step 3 of the GPU route,
//! docs/porting/gpu-rendering.md, #623): the GLSL ES 3.0 of their WGSL module, translated by naga
//! at build time, so the browser module carries no translator. The stages of the library's effects
//! are in this crate (`build.rs`, `ferroui_build::shader_effects`); an effect of an application
//! brings its own (`ShaderEffectDescription::with_glsl`).
//!
//! A program is linked once per context and type, when an effect first needs it. A frame writes
//! the uniforms of the effect and of the pass into two uniform buffers and draws one triangle.
//! The images of a page transition are uploaded once per context and version, and released after
//! their last handle is dropped.
//!
//! An effect without stages (an application's that has none) stays on its CPU reference, as does a
//! program that does not compile (logged once).

use super::frame_textures::VelloWebGlTexture;
use super::layer_programs::compile;
use crate::effect_uniforms::fill_shader_uniforms;
use ferroui_base::media::effects::{EffectImage, EffectImageWeak, ImmutableCustomEffect, MAX_SHADER_CHILDREN};
use std::any::TypeId;
use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;
use web_sys::{
    WebGl2RenderingContext as Gl, WebGlBuffer, WebGlProgram, WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
};

/// The uniform buffer binding point of the block of the effect.
const UNIFORMS_POINT: u32 = 0;
/// The uniform buffer binding point of the block of the pass.
const PASS_POINT: u32 = 1;
/// The size of the block of the pass: one `vec4`.
const PASS_SIZE: usize = 16;

/// A shader effect of the library in GLSL ES 3.0, with what its stages bind.
struct LibraryShader {
    name: &'static str,
    /// The hash of its WGSL module: an application's effect of the same name is not this one.
    module_hash: u64,
    vertex: &'static str,
    fragment: &'static str,
    uniforms_block: &'static str,
    pass_block: &'static str,
    content: &'static str,
    children: &'static [&'static str],
    uniforms_size: u32,
    offsets: &'static [u32],
}

static LIBRARY: &[LibraryShader] = include!(concat!(env!("OUT_DIR"), "/web_gl_shader_effects.rs"));

/// The FNV-1a hash of the text of a module (`ferroui_build::shader_module_hash`).
fn module_hash(module: &str) -> u64 {
    module.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3))
}

/// What a program is linked from.
struct Source {
    vertex: Cow<'static, str>,
    fragment: Cow<'static, str>,
    uniforms_block: Cow<'static, str>,
    pass_block: Cow<'static, str>,
    content: Cow<'static, str>,
    children: Vec<Cow<'static, str>>,
    uniforms_size: u32,
    offsets: Vec<u32>,
}

/// The stages of `effect`: its own, or those of the library for one of its effects.
fn source_of(effect: &ImmutableCustomEffect) -> Option<Source> {
    let description = effect.shader()?;
    if let Some(stages) = &description.glsl {
        // Stages generated from another version of the effect would read the wrong uniforms.
        if stages.children.len() != description.children.len()
            || stages.uniform_offsets.len() != effect.shader_uniform_layout().len()
        {
            super::log_render_failure(&format!("the GLSL of {} does not match its description", effect.name()));
            return None;
        }
        return Some(Source {
            vertex: stages.vertex.clone(),
            fragment: stages.fragment.clone(),
            uniforms_block: stages.uniforms_block.clone(),
            pass_block: stages.pass_block.clone(),
            content: stages.content.clone(),
            children: stages.children.clone(),
            uniforms_size: stages.uniforms_size,
            offsets: stages.uniform_offsets.clone(),
        });
    }
    let shader = LIBRARY.iter().find(|shader| shader.name == effect.name())?;
    if module_hash(&description.wgsl_module()) != shader.module_hash {
        return None;
    }
    Some(Source {
        vertex: shader.vertex.into(),
        fragment: shader.fragment.into(),
        uniforms_block: shader.uniforms_block.into(),
        pass_block: shader.pass_block.into(),
        content: shader.content.into(),
        children: shader.children.iter().map(|name| Cow::Borrowed(*name)).collect(),
        uniforms_size: shader.uniforms_size,
        offsets: shader.offsets.to_vec(),
    })
}

/// A linked program with its buffers.
struct Compiled {
    program: WebGlProgram,
    uniforms: WebGlBuffer,
    pass: WebGlBuffer,
    content: Option<WebGlUniformLocation>,
    /// The samplers of the children; `None` for one the program does not read.
    children: Vec<Option<WebGlUniformLocation>>,
    offsets: Vec<u32>,
    /// The bytes of the block of the effect, kept so that a frame allocates nothing.
    block: Vec<u8>,
}

fn link(gl: &Gl, source: &Source) -> Result<WebGlProgram, String> {
    let vertex = compile(gl, Gl::VERTEX_SHADER, &source.vertex)?;
    let fragment = match compile(gl, Gl::FRAGMENT_SHADER, &source.fragment) {
        Ok(fragment) => fragment,
        Err(error) => {
            gl.delete_shader(Some(&vertex));
            return Err(error);
        }
    };
    let program = gl.create_program().ok_or("the context did not create a program")?;
    gl.attach_shader(&program, &vertex);
    gl.attach_shader(&program, &fragment);
    gl.link_program(&program);
    // The program keeps what it was linked from.
    gl.delete_shader(Some(&vertex));
    gl.delete_shader(Some(&fragment));
    if gl.get_program_parameter(&program, Gl::LINK_STATUS).as_bool().unwrap_or(false) {
        return Ok(program);
    }
    let log = gl.get_program_info_log(&program).unwrap_or_default();
    gl.delete_program(Some(&program));
    Err(log)
}

/// A uniform buffer of `size` bytes.
fn uniform_buffer(gl: &Gl, size: usize) -> Result<WebGlBuffer, String> {
    let buffer = gl.create_buffer().ok_or("the context did not create a buffer")?;
    gl.bind_buffer(Gl::UNIFORM_BUFFER, Some(&buffer));
    gl.buffer_data_with_i32(Gl::UNIFORM_BUFFER, size as i32, Gl::DYNAMIC_DRAW);
    gl.bind_buffer(Gl::UNIFORM_BUFFER, None);
    Ok(buffer)
}

fn compiled(gl: &Gl, source: &Source) -> Result<Compiled, String> {
    let program = link(gl, source)?;
    // A block the program does not read has no index, and nothing to bind.
    for (name, point) in [(&source.uniforms_block, UNIFORMS_POINT), (&source.pass_block, PASS_POINT)] {
        let index = gl.get_uniform_block_index(&program, name);
        if index != Gl::INVALID_INDEX {
            gl.uniform_block_binding(&program, index, point);
        }
    }
    let size = source.uniforms_size as usize;
    let buffers = uniform_buffer(gl, size).and_then(|uniforms| match uniform_buffer(gl, PASS_SIZE) {
        Ok(pass) => Ok((uniforms, pass)),
        Err(error) => {
            gl.delete_buffer(Some(&uniforms));
            Err(error)
        }
    });
    let (uniforms, pass) = match buffers {
        Ok(buffers) => buffers,
        Err(error) => {
            gl.delete_program(Some(&program));
            return Err(error);
        }
    };
    let content = gl.get_uniform_location(&program, &source.content);
    let children = source.children.iter().map(|name| gl.get_uniform_location(&program, name)).collect();
    Ok(Compiled { program, uniforms, pass, content, children, offsets: source.offsets.clone(), block: vec![0; size] })
}

/// An image of a page transition uploaded to the context.
struct ImageTexture {
    handle: EffectImageWeak,
    image: u64,
    version: i32,
    texture: WebGlTexture,
}

/// The programs of the shader effects of a context, and the images they sample.
#[derive(Default)]
pub(super) struct ShaderPrograms {
    vertex_array: Option<WebGlVertexArrayObject>,
    /// By the type of the effect; `None` for an effect drawn by its CPU reference.
    programs: HashMap<TypeId, Option<Compiled>>,
    images: Vec<ImageTexture>,
    /// The transparent texture an empty image, or one that cannot be uploaded, samples.
    empty: Option<WebGlTexture>,
    /// `EffectImage::release_generation` when the images were last looked through.
    released: u64,
}

/// A texture of `width` by `height` premultiplied texels, read by texel.
fn image_upload(gl: &Gl, width: u32, height: u32, rgba: &[u8]) -> Option<WebGlTexture> {
    let texture = gl.create_texture()?;
    gl.active_texture(Gl::TEXTURE0);
    gl.bind_texture(Gl::TEXTURE_2D, Some(&texture));
    for (name, value) in [
        (Gl::TEXTURE_MIN_FILTER, Gl::NEAREST),
        (Gl::TEXTURE_MAG_FILTER, Gl::NEAREST),
        (Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE),
        (Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE),
    ] {
        gl.tex_parameteri(Gl::TEXTURE_2D, name, value as i32);
    }
    let uploaded = gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
        Gl::TEXTURE_2D,
        0,
        Gl::RGBA8 as i32,
        width as i32,
        height as i32,
        0,
        Gl::RGBA,
        Gl::UNSIGNED_BYTE,
        Some(rgba),
    );
    gl.bind_texture(Gl::TEXTURE_2D, None);
    if uploaded.is_err() {
        gl.delete_texture(Some(&texture));
        return None;
    }
    Some(texture)
}

impl ShaderPrograms {
    /// Whether `effect` is drawn on the context, its program linked now if it was not.
    pub(super) fn prepare(&mut self, gl: &Gl, effect: &ImmutableCustomEffect) -> bool {
        let type_id = effect.value_type_id();
        if let Some(entry) = self.programs.get(&type_id) {
            return entry.is_some();
        }
        let entry = source_of(effect).and_then(|source| match compiled(gl, &source) {
            Ok(compiled) => Some(compiled),
            Err(error) => {
                super::log_render_failure(&format!("the program of {} did not compile: {error}", effect.name()));
                None
            }
        });
        let ready = entry.is_some();
        self.programs.insert(type_id, entry);
        ready
    }

    /// The transparent texture of one texel.
    fn empty_texture(&mut self, gl: &Gl) -> Option<WebGlTexture> {
        if self.empty.is_none() {
            self.empty = image_upload(gl, 1, 1, &[0; 4]);
        }
        self.empty.clone()
    }

    /// The texture of `image`, uploaded once per version. An empty image, or one the context does
    /// not take, samples as transparent, as on the device.
    fn image_texture(&mut self, gl: &Gl, image: &EffectImage) -> Option<WebGlTexture> {
        if image.is_empty() {
            return self.empty_texture(gl);
        }
        let (id, version) = (image.id(), image.version());
        if let Some(found) = self.images.iter().find(|entry| entry.image == id && entry.version == version) {
            return Some(found.texture.clone());
        }
        // A new version of the bitmap: the old pixels go.
        self.images.retain(|entry| {
            let keep = entry.image != id;
            if !keep {
                gl.delete_texture(Some(&entry.texture));
            }
            keep
        });
        let uploaded = image.pixels().and_then(|pixels| {
            let (width, height) = (pixels.width() as u32, pixels.height() as u32);
            if width == 0 || height == 0 {
                return None;
            }
            image_upload(gl, width, height, pixels.rgba())
        });
        match uploaded {
            Some(texture) => {
                let handle = image.downgrade();
                self.images.push(ImageTexture { handle, image: id, version, texture: texture.clone() });
                Some(texture)
            }
            None => self.empty_texture(gl),
        }
    }

    /// Lets go of the textures of images whose last handle was dropped.
    fn release_dropped_images(&mut self, gl: &Gl) {
        let released = EffectImage::release_generation();
        if released == self.released {
            return;
        }
        self.released = released;
        self.images.retain(|entry| {
            let alive = entry.handle.is_alive();
            if !alive {
                gl.delete_texture(Some(&entry.texture));
            }
            alive
        });
    }

    /// The pass of `effect`: `content` (upright, one texel per pixel) over `width` by `height`
    /// pixels, whose top left is `origin` of the area of `size` pixels, written into `target`,
    /// its first row the top one. `false`, drawing nothing, without a program or when the
    /// uniforms or images do not match it.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn pass(
        &mut self,
        gl: &Gl,
        effect: &ImmutableCustomEffect,
        content: &Rc<VelloWebGlTexture>,
        target: &Rc<VelloWebGlTexture>,
        (width, height): (u32, u32),
        origin: [f32; 2],
        size: [f32; 2],
        scale: f32,
    ) -> bool {
        if self.vertex_array.is_none() {
            self.vertex_array = gl.create_vertex_array();
        }
        let vertex_array = self.vertex_array.clone();
        self.release_dropped_images(gl);
        let type_id = effect.value_type_id();
        let Some(Some(program)) = self.programs.get(&type_id) else {
            return false;
        };
        let expected = program.children.len();
        // The textures of the images, in the order of the children.
        let mut images: [Option<WebGlTexture>; MAX_SHADER_CHILDREN] = std::array::from_fn(|_| None);
        let mut count = 0;
        effect.for_each_shader_child(&mut |_, image| {
            if let Some(slot) = images.get_mut(count) {
                *slot = self.image_texture(gl, image);
            }
            count += 1;
        });
        if count != expected || images[..expected.min(MAX_SHADER_CHILDREN)].iter().any(Option::is_none) {
            return false;
        }
        let Some(Some(compiled)) = self.programs.get_mut(&type_id) else {
            return false;
        };
        compiled.block.fill(0);
        if !fill_shader_uniforms(&mut compiled.block, &compiled.offsets, effect, size, scale) {
            return false;
        }
        let mut pass_bytes = [0u8; PASS_SIZE];
        let pass_floats = [origin[0], origin[1], width as f32, height as f32];
        for (slot, value) in pass_bytes.chunks_exact_mut(4).zip(pass_floats) {
            slot.copy_from_slice(&value.to_ne_bytes());
        }
        gl.bind_buffer(Gl::UNIFORM_BUFFER, Some(&compiled.uniforms));
        gl.buffer_sub_data_with_i32_and_u8_array(Gl::UNIFORM_BUFFER, 0, &compiled.block);
        gl.bind_buffer(Gl::UNIFORM_BUFFER, Some(&compiled.pass));
        gl.buffer_sub_data_with_i32_and_u8_array(Gl::UNIFORM_BUFFER, 0, &pass_bytes);
        gl.bind_buffer(Gl::UNIFORM_BUFFER, None);

        gl.use_program(Some(&compiled.program));
        gl.uniform1i(compiled.content.as_ref(), 0);
        gl.bind_buffer_base(Gl::UNIFORM_BUFFER, UNIFORMS_POINT, Some(&compiled.uniforms));
        gl.bind_buffer_base(Gl::UNIFORM_BUFFER, PASS_POINT, Some(&compiled.pass));

        let tests = [Gl::BLEND, Gl::SCISSOR_TEST, Gl::DEPTH_TEST, Gl::STENCIL_TEST];
        let enabled = tests.map(|test| gl.is_enabled(test));
        for test in tests {
            gl.disable(test);
        }
        gl.bind_framebuffer(Gl::FRAMEBUFFER, Some(&target.framebuffer));
        gl.viewport(0, 0, target.width as i32, target.height as i32);
        gl.active_texture(Gl::TEXTURE0);
        gl.bind_texture(Gl::TEXTURE_2D, Some(&content.texture));
        for (child, location) in compiled.children.iter().enumerate() {
            let unit = child as u32 + 1;
            gl.uniform1i(location.as_ref(), unit as i32);
            gl.active_texture(Gl::TEXTURE0 + unit);
            gl.bind_texture(Gl::TEXTURE_2D, images[child].as_ref());
        }
        gl.bind_vertex_array(vertex_array.as_ref());
        gl.draw_arrays(Gl::TRIANGLES, 0, 3);

        gl.bind_vertex_array(None);
        for child in (0..compiled.children.len()).rev() {
            gl.active_texture(Gl::TEXTURE0 + child as u32 + 1);
            gl.bind_texture(Gl::TEXTURE_2D, None);
        }
        gl.active_texture(Gl::TEXTURE0);
        gl.bind_texture(Gl::TEXTURE_2D, None);
        gl.bind_buffer_base(Gl::UNIFORM_BUFFER, UNIFORMS_POINT, None);
        gl.bind_buffer_base(Gl::UNIFORM_BUFFER, PASS_POINT, None);
        gl.use_program(None);
        gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
        gl.viewport(0, 0, gl.drawing_buffer_width(), gl.drawing_buffer_height());
        for (test, enabled) in tests.into_iter().zip(enabled) {
            if enabled {
                gl.enable(test);
            }
        }
        true
    }

    /// Deletes the programs, buffers and images: the context is released.
    pub(super) fn dispose(&mut self, gl: &Gl) {
        for (_, entry) in self.programs.drain() {
            if let Some(compiled) = entry {
                gl.delete_program(Some(&compiled.program));
                gl.delete_buffer(Some(&compiled.uniforms));
                gl.delete_buffer(Some(&compiled.pass));
            }
        }
        for image in self.images.drain(..) {
            gl.delete_texture(Some(&image.texture));
        }
        if let Some(empty) = self.empty.take() {
            gl.delete_texture(Some(&empty));
        }
        if let Some(vertex_array) = self.vertex_array.take() {
            gl.delete_vertex_array(Some(&vertex_array));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_what_the_programs_bind() {
        assert_eq!(10, LIBRARY.len());
        for shader in LIBRARY {
            assert!(shader.vertex.starts_with("#version 300 es"), "{}", shader.name);
            assert!(shader.fragment.contains(shader.content), "{}", shader.name);
            assert!(shader.fragment.contains(shader.pass_block), "{}", shader.name);
            assert!(shader.children.iter().all(|child| shader.fragment.contains(child)), "{}", shader.name);
            assert_eq!(0, shader.uniforms_size % 16, "{}", shader.name);
            assert!(shader.offsets.iter().all(|offset| *offset < shader.uniforms_size), "{}", shader.name);
        }
    }

    #[test]
    fn the_hash_of_a_module_is_fnv_1a() {
        assert_eq!(0xcbf2_9ce4_8422_2325, module_hash(""));
        assert_eq!(0xaf63_dc4c_8601_ec8c, module_hash("a"));
    }
}
