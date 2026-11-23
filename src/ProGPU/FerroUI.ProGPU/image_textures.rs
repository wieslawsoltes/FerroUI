//! The textures of the bitmaps a device draws (docs/porting/progpu-backend.md, step R6).
//!
//! A bitmap of pixels used to travel in the stream of every scene that drew it, and the engine
//! uploaded it again with each generation. Here it is uploaded once per version into a texture
//! of the device, and a scene names that texture as an external image. The identity the engine
//! sees is still the one the builder gives the resource, ascending within the scene; this table
//! is what maps a bitmap to its texture from frame to frame, so the rule of ascending
//! identifiers holds whatever order a frame draws its bitmaps in.

use crate::bitmap::ProGpuBitmap;
use crate::device::ProGpuDevice;
use crate::gpu::GpuTexture;
use ferroui_base::platform::{AlphaFormat, IBitmapImpl, PixelFormat};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Weak;

/// The most bytes of textures kept for bitmaps. Past it the textures used longest ago go.
const BUDGET_BYTES: usize = 256 << 20;
/// The largest side of a bitmap kept as a texture; a larger one travels in the stream.
const MAX_SIDE: i32 = 8192;

struct Entry {
    /// The pixels of the bitmap. It also keeps their address from being reused for another
    /// bitmap for as long as this entry is in the table.
    owner: Weak<dyn Any + Send + Sync>,
    version: i32,
    texture: Rc<GpuTexture>,
    last_frame: u64,
}

impl Entry {
    fn bytes(&self) -> usize {
        self.texture.width() as usize * self.texture.height() as usize * 4
    }
}

/// The table of a device.
#[derive(Default)]
pub(crate) struct ImageTextures {
    entries: RefCell<HashMap<usize, Entry>>,
    frame: Cell<u64>,
}

/// The pixels of `bitmap` as a texture holds them: `RGBA`, premultiplied, rows without padding.
fn texture_pixels(bitmap: &ProGpuBitmap, width: usize, height: usize) -> Vec<u8> {
    let row_bytes = bitmap.row_bytes();
    let bgra = bitmap.pixel_format() == PixelFormat::BGRA8888;
    let alpha = bitmap.pixel_alpha_format();
    bitmap.with_pixels(|pixels| {
        let mut out = Vec::with_capacity(width * height * 4);
        for y in 0..height {
            let start = y * row_bytes;
            for pixel in pixels[start..start + width * 4].chunks_exact(4) {
                let (r, g, b, a) =
                    if bgra { (pixel[2], pixel[1], pixel[0], pixel[3]) } else { (pixel[0], pixel[1], pixel[2], pixel[3]) };
                match alpha {
                    AlphaFormat::Opaque => out.extend_from_slice(&[r, g, b, 255]),
                    AlphaFormat::Premul => out.extend_from_slice(&[r, g, b, a]),
                    AlphaFormat::Unpremul => {
                        let times = |c: u8| ((u32::from(c) * u32::from(a) + 127) / 255) as u8;
                        out.extend_from_slice(&[times(r), times(g), times(b), a]);
                    }
                }
            }
        }
        out
    })
}

impl ImageTextures {
    /// A frame begins: textures of bitmaps that are gone leave, and the table is brought under
    /// its budget. A texture a frame holds stays alive through its own reference.
    pub(crate) fn begin_frame(&self) {
        self.frame.set(self.frame.get() + 1);
        let mut entries = self.entries.borrow_mut();
        entries.retain(|_, entry| entry.owner.strong_count() > 0);
        let mut total: usize = entries.values().map(Entry::bytes).sum();
        if total <= BUDGET_BYTES {
            return;
        }
        let mut oldest: Vec<(u64, usize, usize)> =
            entries.iter().map(|(key, entry)| (entry.last_frame, *key, entry.bytes())).collect();
        oldest.sort_unstable();
        for (_, key, bytes) in oldest {
            if total <= BUDGET_BYTES {
                break;
            }
            entries.remove(&key);
            total -= bytes;
        }
    }

    /// The texture of `bitmap` at its current version, uploaded if it is not. `None` for a
    /// bitmap that stays in the stream: too large, or without pixels.
    pub(crate) fn texture_of(&self, device: &Rc<ProGpuDevice>, bitmap: &ProGpuBitmap) -> Option<Rc<GpuTexture>> {
        let size = bitmap.pixel_size();
        if size.width <= 0 || size.height <= 0 || size.width > MAX_SIDE || size.height > MAX_SIDE {
            return None;
        }
        let (width, height) = (size.width as usize, size.height as usize);
        let (key, owner) = bitmap.pixels_identity();
        let version = bitmap.version();
        let frame = self.frame.get();
        let mut entries = self.entries.borrow_mut();
        if let Some(entry) = entries.get_mut(&key) {
            if Rc::ptr_eq(entry.texture.device(), device) {
                if entry.version != version {
                    entry.texture.write(&texture_pixels(bitmap, width, height));
                    entry.version = version;
                }
                entry.last_frame = frame;
                return Some(entry.texture.clone());
            }
        }
        let texture = GpuTexture::new_image(device, width as u32, height as u32);
        texture.write(&texture_pixels(bitmap, width, height));
        entries.insert(key, Entry { owner, version, texture: texture.clone(), last_frame: frame });
        Some(texture)
    }

    /// How many textures are kept (for the tests).
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.borrow().len()
    }
}
