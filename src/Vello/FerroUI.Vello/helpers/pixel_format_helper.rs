//! Conversions between the pixel formats of the contract and the one format
//! the renderers of the Vello project draw: premultiplied RGBA, eight bits a
//! channel, rows without padding.

use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::PixelSize;
use peniko::{Blob, ImageAlphaType, ImageData, ImageFormat};
use std::sync::Arc;

/// The pixel formats the backend reads and writes.
pub fn is_supported(format: PixelFormat) -> bool {
    format == PixelFormat::RGBA8888 || format == PixelFormat::BGRA8888
}

/// # Panics
/// Panics for a pixel format the backend does not read or write.
fn assert_supported(format: PixelFormat) {
    if !is_supported(format) {
        panic!("Unsupported pixel format: {format:?}. The Vello backend reads and writes RGBA8888 and BGRA8888.");
    }
}

/// The scene size of a pixel size.
///
/// # Panics
/// Panics for a size the renderers of the Vello project cannot draw: they
/// count pixels in 16 bits.
pub fn scene_size(size: PixelSize) -> (u16, u16) {
    match (u16::try_from(size.width.max(1)), u16::try_from(size.height.max(1))) {
        (Ok(width), Ok(height)) => (width, height),
        _ => panic!("Unable to create a render target of {}x{} pixels: more than 65535 in a direction", size.width, size.height),
    }
}

/// Reads pixels in a format of the contract into premultiplied RGBA without
/// padding.
///
/// `data` holds `row_bytes` bytes a row; rows that are missing read as
/// transparent.
pub fn to_premul_rgba(
    data: &[u8],
    row_bytes: usize,
    size: PixelSize,
    format: PixelFormat,
    alpha_format: AlphaFormat,
) -> Vec<u8> {
    assert_supported(format);

    let (width, height) = (size.width.max(0) as usize, size.height.max(0) as usize);
    let swap = format == PixelFormat::BGRA8888;
    let mut rgba = vec![0u8; width * height * 4];

    for (y, target_row) in rgba.chunks_exact_mut(width.max(1) * 4).enumerate() {
        let Some(source_row) = data.get(y * row_bytes..y * row_bytes + width * 4) else {
            break;
        };

        for (target, source) in target_row.chunks_exact_mut(4).zip(source_row.chunks_exact(4)) {
            let (mut r, mut g, mut b, mut a) = (source[0], source[1], source[2], source[3]);
            if swap {
                std::mem::swap(&mut r, &mut b);
            }

            match alpha_format {
                AlphaFormat::Premul => {}
                AlphaFormat::Opaque => a = 255,
                AlphaFormat::Unpremul => {
                    let premultiply = |c: u8| ((c as u32 * a as u32 + 127) / 255) as u8;
                    (r, g, b) = (premultiply(r), premultiply(g), premultiply(b));
                }
            }

            target.copy_from_slice(&[r, g, b, a]);
        }
    }

    rgba
}

/// Writes premultiplied RGBA pixels without padding as pixels in a format
/// of the contract, `row_bytes` bytes a row. The padding of the rows is left
/// as it is.
pub fn from_premul_rgba(
    rgba: &[u8],
    data: &mut [u8],
    row_bytes: usize,
    size: PixelSize,
    format: PixelFormat,
    alpha_format: AlphaFormat,
) {
    assert_supported(format);

    let width = size.width.max(0) as usize;
    let swap = format == PixelFormat::BGRA8888;

    for (y, source_row) in rgba.chunks_exact(width.max(1) * 4).enumerate() {
        let Some(target_row) = data.get_mut(y * row_bytes..y * row_bytes + width * 4) else {
            break;
        };

        for (target, source) in target_row.chunks_exact_mut(4).zip(source_row.chunks_exact(4)) {
            let (mut r, mut g, mut b, mut a) = (source[0], source[1], source[2], source[3]);

            match alpha_format {
                AlphaFormat::Premul => {}
                AlphaFormat::Opaque => a = 255,
                AlphaFormat::Unpremul => {
                    if a != 0 && a != 255 {
                        let unpremultiply = |c: u8| ((c as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8;
                        (r, g, b) = (unpremultiply(r), unpremultiply(g), unpremultiply(b));
                    }
                }
            }

            if swap {
                std::mem::swap(&mut r, &mut b);
            }

            target.copy_from_slice(&[r, g, b, a]);
        }
    }
}

/// Premultiplied RGBA pixels without padding as an image the scene of a
/// frame can paint with.
pub fn to_image(rgba: Vec<u8>, size: PixelSize) -> ImageData {
    ImageData {
        data: Blob::new(Arc::new(rgba)),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::AlphaPremultiplied,
        width: size.width.max(0) as u32,
        height: size.height.max(0) as u32,
    }
}
