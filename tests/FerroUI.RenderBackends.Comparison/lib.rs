//! The comparison harness of the render backends.
//!
//! The same scenes are drawn through the platform contracts by the Skia
//! backend (its raster path) and by the Vello backend (each rendering mode
//! that is built), into render target bitmaps of the same size, and the
//! pixels are compared: for every scene the share of pixels that differ by
//! more than a tolerance (the two renderers anti-alias edges differently, so
//! pixels on an edge differ a little and are not counted). The geometry
//! queries of the contracts (bounds, lengths, hit tests) are compared for a
//! set of shapes in numbers.
//!
//! The tests print both comparisons as the tables of
//! `docs/porting/vello-backend.md` (section 8) and fail when a scene or a
//! shape is further from the Skia backend than the bound recorded for it:
//! the progress of the Vello backend is these numbers.
//!
//! This is the one crate that links both backends; an application has one.

pub mod effect_scenes;
pub mod geometries;
pub mod modes;
pub mod scenes;
pub mod text;

use ferroui_base::platform::{IDrawingContextImpl, IPlatformRenderInterface, IReadableBitmapImpl, PixelFormat};
use ferroui_base::{PixelSize, Vector};
use ferroui_vello::{VelloOptions, VelloRenderingMode};
use std::rc::Rc;

/// A render backend under comparison.
pub struct Backend {
    /// The name the tables show.
    pub name: &'static str,
    /// The render interface of the backend.
    pub interface: Rc<dyn IPlatformRenderInterface>,
}

impl Backend {
    /// The Skia backend. Render target bitmaps are drawn by its raster path.
    pub fn skia() -> Self {
        Self { name: "Skia raster", interface: Rc::new(ferroui_skia::PlatformRenderInterface::default()) }
    }

    /// The Vello backend in one rendering mode, without another to fall back
    /// to: a mode that is not built fails when it is asked to draw.
    pub fn vello(mode: VelloRenderingMode) -> Self {
        let name = match mode {
            VelloRenderingMode::Cpu => "Vello CPU",
            VelloRenderingMode::Hybrid => "Vello hybrid",
            VelloRenderingMode::Gpu => "Vello GPU",
        };

        Self {
            name,
            interface: Rc::new(ferroui_vello::PlatformRenderInterface::new(VelloOptions::with_rendering_mode(mode))),
        }
    }

    /// The rendering modes of the Vello backend that are built and can be
    /// compared. The hybrid and the GPU mode join the list with their
    /// stages.
    pub fn vello_modes() -> Vec<VelloRenderingMode> {
        [VelloRenderingMode::Cpu, VelloRenderingMode::Hybrid, VelloRenderingMode::Gpu]
            .into_iter()
            .filter(|mode| ferroui_vello::scene::try_create_scene_sink(*mode, 1, 1).is_ok())
            .collect()
    }
}

/// The pixels of a rendered scene: premultiplied RGBA, four bytes a pixel,
/// rows without padding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pixels {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

/// Draws a scene with a backend into a render target bitmap of `size`
/// pixels at 96 DPI and reads the pixels back.
pub fn render(backend: &Backend, size: PixelSize, scene: &dyn Fn(&Backend, &mut dyn IDrawingContextImpl)) -> Pixels {
    let bitmap = backend.interface.create_render_target_bitmap(size, Vector::new(96.0, 96.0));

    let mut context = bitmap.create_drawing_context();
    scene(backend, &mut *context);
    context.dispose();

    let pixels = read_pixels(&*bitmap);
    bitmap.dispose();
    pixels
}

/// The pixels of a bitmap as premultiplied RGBA.
///
/// # Panics
/// Panics for a bitmap that is not in one of the two 32 bit formats with
/// premultiplied alpha, which is what both backends draw into.
pub fn read_pixels(bitmap: &dyn IReadableBitmapImpl) -> Pixels {
    let framebuffer = bitmap.lock();
    let size = framebuffer.size();
    let (width, height) = (size.width as usize, size.height as usize);
    let row_bytes = framebuffer.row_bytes() as usize;
    let format = framebuffer.format();
    assert!(format == PixelFormat::RGBA8888 || format == PixelFormat::BGRA8888, "an unexpected format: {format:?}");
    assert_eq!(ferroui_base::platform::AlphaFormat::Premul, framebuffer.alpha_format());

    let mut rgba = Vec::with_capacity(width * height * 4);
    framebuffer.with_data(&mut |data| {
        for y in 0..height {
            for pixel in data[y * row_bytes..y * row_bytes + width * 4].chunks_exact(4) {
                if format == PixelFormat::BGRA8888 {
                    rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
                } else {
                    rgba.extend_from_slice(pixel);
                }
            }
        }
    });
    framebuffer.dispose();

    Pixels { width, height, rgba }
}

/// How far the pixels of two renderings of a scene are apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Difference {
    /// The share of pixels of which a channel differs by more than the
    /// tolerance, in percent.
    pub share: f64,
    /// The largest difference of a channel of a pixel, of 255.
    pub largest: u8,
    /// The mean difference of the channels of all pixels, of 255.
    pub mean: f64,
}

/// The difference of a channel up to which two pixels count as the same: an
/// eighth of the range. What two anti-aliasing methods make of the same
/// edge is within it; a shape that is missing, moved by a pixel or of
/// another color is not.
pub const TOLERANCE: u8 = 32;

/// Compares two renderings of the same size.
pub fn compare(a: &Pixels, b: &Pixels) -> Difference {
    assert_eq!((a.width, a.height), (b.width, b.height), "the renderings have the same size");

    let (mut differing, mut largest, mut sum) = (0usize, 0u8, 0u64);
    for (pixel_a, pixel_b) in a.rgba.chunks_exact(4).zip(b.rgba.chunks_exact(4)) {
        let mut pixel_largest = 0u8;
        for (channel_a, channel_b) in pixel_a.iter().zip(pixel_b) {
            let difference = channel_a.abs_diff(*channel_b);
            pixel_largest = pixel_largest.max(difference);
            sum += difference as u64;
        }
        if pixel_largest > TOLERANCE {
            differing += 1;
        }
        largest = largest.max(pixel_largest);
    }

    let pixel_count = (a.width * a.height).max(1);
    Difference {
        share: 100.0 * differing as f64 / pixel_count as f64,
        largest,
        mean: sum as f64 / (pixel_count * 4) as f64,
    }
}

/// The pixels of two renderings of one scene by one backend that may differ,
/// and by how much: none, but for the GPU mode of the Vello backend.
///
/// The compute renderer of that mode is not exact to the last bit from one
/// render to the next: its stages run in parallel over the tiles and the
/// segments of a scene and hand out memory with atomic counters, so the
/// order in which the segments of a tile are summed up to the coverage of a
/// pixel is the order in which the threads of the GPU got there, and the
/// sum, in single precision, differs in its last bit. A pixel in ten
/// thousand of a scene of text (many short segments) comes out one digit of
/// a color apart, in some renders and not in others (measured: 1 to 5
/// pixels of 40 000 in one of four renders of a text scene, also with a new
/// renderer for every render; never in a scene without curves). It is not
/// state that is carried from a render to the next: that was found and
/// closed in the sink of the mode (the places of images in the atlas of the
/// renderer, `ImagePlace` in `vello_gpu_scene_sink.rs`), and a difference of
/// more than one digit, or in more pixels than this, fails.
pub const GPU_MODE_REPEAT_PIXELS: usize = 16;

/// Checks that a backend drew a scene the same way twice: the same pixels,
/// and for the GPU mode of the Vello backend at most
/// [`GPU_MODE_REPEAT_PIXELS`] pixels one digit of a color apart.
pub fn drawn_the_same_way_twice(backend: &Backend, first: &Pixels, second: &Pixels) -> Result<(), String> {
    if first == second {
        return Ok(());
    }

    let differing = first.rgba.chunks_exact(4).zip(second.rgba.chunks_exact(4)).filter(|(a, b)| a != b).count();
    let largest = first.rgba.iter().zip(&second.rgba).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
    if backend.name == "Vello GPU" && largest <= 1 && differing <= GPU_MODE_REPEAT_PIXELS {
        return Ok(());
    }

    Err(format!("{differing} pixels differ, by up to {largest} of 255"))
}

#[cfg(test)]
mod application_tests;
#[cfg(test)]
mod codec_tests;
#[cfg(test)]
mod effect_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod text_tests;
