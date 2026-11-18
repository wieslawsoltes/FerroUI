//! Textures of the WebGL2 context that a scene paints with (step 3 of the GPU route,
//! docs/porting/gpu-rendering.md, "Vello: what is missing and the GPU route", #623).
//!
//! The renderer of `vello_gpu` over WebGL2 paints with a texture of the context it is given a
//! binding for (`WebGlTextureBindings`, an `ImageSource::ExternalTexture` of the scene): an image
//! of the scene is one, uploaded by [`VelloWebGlGpu`](super::VelloWebGlGpu). A
//! [`VelloWebGlTexture`] is another, written on the context and never read back: the content of a
//! layer copied from the drawing buffer after it was rendered there, or the result of a pass of
//! `layer_programs.rs`. Each has a framebuffer, so a pass draws into it.
//!
//! The textures are pooled by size, rounded up to [`POOL_STEP`] pixels: a texture is the pool's
//! again when nothing but the pool holds it (the scene that paints with it was rendered and
//! dropped), so steady frames allocate no texture after the first.

use std::rc::Rc;
use vello_gpu::TextureId;
use web_sys::{WebGl2RenderingContext as Gl, WebGlFramebuffer, WebGlTexture};

/// The step the sizes of the pool are rounded up to.
pub(crate) const POOL_STEP: u32 = 32;

/// The identities of the textures of the pool are apart from those of images (the identities
/// of their pixels, a counter that starts at 1): the highest bit is set.
const TEXTURE_ID_BASE: u64 = 1 << 63;

/// A texture of the WebGL2 context of a canvas, with a framebuffer that draws into it:
/// premultiplied RGBA, its first row the top one (as the renderer samples a bound texture).
pub struct VelloWebGlTexture {
    pub(crate) texture: WebGlTexture,
    pub(crate) framebuffer: WebGlFramebuffer,
    /// The size of the texture (a size of the pool).
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// The identity a scene paints it by.
    pub(crate) id: TextureId,
}

impl VelloWebGlTexture {
    /// The size of the texture in pixels.
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl std::fmt::Debug for VelloWebGlTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VelloWebGlTexture").field("width", &self.width).field("height", &self.height).finish()
    }
}

/// The size of the pool for an area of `width` by `height` pixels.
pub(crate) fn pooled_size(width: u32, height: u32, max_texture_size: u32) -> (u32, u32) {
    let round = |value: u32| value.max(1).div_ceil(POOL_STEP).saturating_mul(POOL_STEP).min(max_texture_size.max(1));
    (round(width), round(height))
}

/// The textures of a context, by size.
#[derive(Default)]
pub(crate) struct TexturePool {
    textures: Vec<Rc<VelloWebGlTexture>>,
    next_id: u64,
    /// How many textures the pool made (steady frames make none).
    allocations: u64,
}

impl TexturePool {
    /// A texture of the pool of `width` by `height` (a size of the pool) that nothing else holds,
    /// or a new one.
    pub(crate) fn acquire(&mut self, gl: &Gl, width: u32, height: u32) -> Result<Rc<VelloWebGlTexture>, String> {
        if let Some(free) =
            self.textures.iter().find(|texture| Rc::strong_count(texture) == 1 && texture.size() == (width, height))
        {
            return Ok(free.clone());
        }

        let texture = gl.create_texture().ok_or("the context did not create a texture")?;
        gl.active_texture(Gl::TEXTURE0);
        gl.bind_texture(Gl::TEXTURE_2D, Some(&texture));
        // Complete without mipmaps: the renderer reads it with `texelFetch`, level 0 alone.
        for (name, value) in [
            (Gl::TEXTURE_MIN_FILTER, Gl::NEAREST),
            (Gl::TEXTURE_MAG_FILTER, Gl::NEAREST),
            (Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE),
            (Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameteri(Gl::TEXTURE_2D, name, value as i32);
        }
        gl.tex_storage_2d(Gl::TEXTURE_2D, 1, Gl::RGBA8, width as i32, height as i32);
        gl.bind_texture(Gl::TEXTURE_2D, None);

        let Some(framebuffer) = gl.create_framebuffer() else {
            gl.delete_texture(Some(&texture));
            return Err("the context did not create a framebuffer".to_string());
        };
        gl.bind_framebuffer(Gl::FRAMEBUFFER, Some(&framebuffer));
        gl.framebuffer_texture_2d(Gl::FRAMEBUFFER, Gl::COLOR_ATTACHMENT0, Gl::TEXTURE_2D, Some(&texture), 0);
        let complete = gl.check_framebuffer_status(Gl::FRAMEBUFFER) == Gl::FRAMEBUFFER_COMPLETE;
        gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
        if !complete {
            gl.delete_framebuffer(Some(&framebuffer));
            gl.delete_texture(Some(&texture));
            return Err(format!("a framebuffer of a texture of {width}x{height} pixels is not complete"));
        }

        self.next_id += 1;
        self.allocations += 1;
        let texture = Rc::new(VelloWebGlTexture {
            texture,
            framebuffer,
            width,
            height,
            id: TextureId(TEXTURE_ID_BASE | self.next_id),
        });
        self.textures.push(texture.clone());
        Ok(texture)
    }

    /// Lets go of the textures of sizes nothing holds beyond `keep` of each size: what a frame
    /// with fewer layers than the one before leaves.
    pub(crate) fn trim(&mut self, gl: &Gl, keep: usize) {
        let mut kept: Vec<(u32, u32, usize)> = Vec::new();
        self.textures.retain(|texture| {
            if Rc::strong_count(texture) > 1 {
                return true;
            }
            let size = texture.size();
            match kept.iter_mut().find(|(width, height, _)| (*width, *height) == size) {
                Some((_, _, count)) if *count >= keep => {
                    gl.delete_framebuffer(Some(&texture.framebuffer));
                    gl.delete_texture(Some(&texture.texture));
                    false
                }
                Some((_, _, count)) => {
                    *count += 1;
                    true
                }
                None => {
                    kept.push((size.0, size.1, 1));
                    true
                }
            }
        });
    }

    /// Deletes every texture of the pool: the context is released.
    pub(crate) fn dispose(&mut self, gl: &Gl) {
        for texture in self.textures.drain(..) {
            gl.delete_framebuffer(Some(&texture.framebuffer));
            gl.delete_texture(Some(&texture.texture));
        }
    }

    /// How many textures the pool made.
    #[allow(dead_code)]
    pub(crate) fn allocations(&self) -> u64 {
        self.allocations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sizes_of_the_pool_are_rounded_up_and_fit_the_context() {
        assert_eq!(pooled_size(1, 1, 4096), (32, 32));
        assert_eq!(pooled_size(33, 64, 4096), (64, 64));
        assert_eq!(pooled_size(0, 100, 4096), (32, 128));
        assert_eq!(pooled_size(5000, 10, 4096), (4096, 32));
    }
}
