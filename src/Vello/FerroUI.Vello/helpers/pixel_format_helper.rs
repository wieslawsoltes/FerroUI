//! Conversions between the pixel formats of the contract and the one format
//! the renderers of the Vello project draw: premultiplied RGBA, eight bits a
//! channel, rows without padding.
//!
//! The formats are those the Skia backend has a color type for: RGBA8888 and
//! BGRA8888 in the three alpha formats, RGB565 (sixteen bits a pixel, the
//! red channel in the high bits of a little-endian word) and RGB32 (red,
//! green, blue and a byte that is not used), which are opaque.

use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::PixelSize;
use peniko::{Blob, ImageAlphaType, ImageData, ImageFormat};
use std::sync::Arc;

/// The pixel formats of bitmaps the backend supports, as the render
/// interface reports them: those of the Skia backend.
pub fn is_supported(format: PixelFormat) -> bool {
    format == PixelFormat::RGB565 || format == PixelFormat::RGBA8888 || format == PixelFormat::BGRA8888
}

/// The bytes of a pixel in a format the backend reads and writes, or `None`
/// for a format it does not.
pub fn try_bytes_per_pixel(format: PixelFormat) -> Option<usize> {
    if format == PixelFormat::RGB565 {
        Some(2)
    } else if format == PixelFormat::RGBA8888 || format == PixelFormat::BGRA8888 || format == PixelFormat::RGB32 {
        Some(4)
    } else {
        None
    }
}

/// The bytes of a pixel in a format the backend reads and writes.
///
/// # Panics
/// Panics for a pixel format the backend does not read or write.
pub fn bytes_per_pixel(format: PixelFormat) -> usize {
    try_bytes_per_pixel(format).unwrap_or_else(|| {
        panic!("Unknown pixel format: {format:?}. The Vello backend reads and writes RGB565, RGBA8888, BGRA8888 and RGB32.")
    })
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

fn premultiply(channel: u8, alpha: u8) -> u8 {
    ((channel as u32 * alpha as u32 + 127) / 255) as u8
}

fn unpremultiply(channel: u8, alpha: u8) -> u8 {
    ((channel as u32 * 255 + alpha as u32 / 2) / alpha as u32).min(255) as u8
}

/// Reads pixels in a format of the contract into premultiplied RGBA without
/// padding.
///
/// `data` holds `row_bytes` bytes a row; rows that are missing read as
/// transparent.
///
/// # Panics
/// Panics for a pixel format the backend does not read or write.
pub fn to_premul_rgba(
    data: &[u8],
    row_bytes: usize,
    size: PixelSize,
    format: PixelFormat,
    alpha_format: AlphaFormat,
) -> Vec<u8> {
    let pixel_bytes = bytes_per_pixel(format);

    let (width, height) = (size.width.max(0) as usize, size.height.max(0) as usize);
    let mut rgba = vec![0u8; width * height * 4];

    for (y, target_row) in rgba.chunks_exact_mut(width.max(1) * 4).enumerate() {
        let Some(source_row) = data.get(y * row_bytes..y * row_bytes + width * pixel_bytes) else {
            break;
        };

        for (target, source) in target_row.chunks_exact_mut(4).zip(source_row.chunks_exact(pixel_bytes)) {
            let pixel = if format == PixelFormat::RGB565 {
                let value = u16::from_le_bytes([source[0], source[1]]);
                let (r, g, b) = ((value >> 11) as u8 & 0x1f, (value >> 5) as u8 & 0x3f, value as u8 & 0x1f);
                // The high bits again in the low ones: the darkest value
                // is 0 and the lightest 255.
                [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2), 255]
            } else if format == PixelFormat::RGB32 {
                [source[0], source[1], source[2], 255]
            } else {
                let (mut r, mut g, mut b, mut a) = (source[0], source[1], source[2], source[3]);
                if format == PixelFormat::BGRA8888 {
                    std::mem::swap(&mut r, &mut b);
                }

                match alpha_format {
                    AlphaFormat::Premul => {}
                    AlphaFormat::Opaque => a = 255,
                    AlphaFormat::Unpremul => (r, g, b) = (premultiply(r, a), premultiply(g, a), premultiply(b, a)),
                }

                [r, g, b, a]
            };

            target.copy_from_slice(&pixel);
        }
    }

    rgba
}

/// Writes premultiplied RGBA pixels without padding as pixels in a format
/// of the contract, `row_bytes` bytes a row. The padding of the rows is left
/// as it is.
///
/// A format without alpha gets the premultiplied colors: what is drawn,
/// over black.
///
/// # Panics
/// Panics for a pixel format the backend does not read or write.
pub fn from_premul_rgba(
    rgba: &[u8],
    data: &mut [u8],
    row_bytes: usize,
    size: PixelSize,
    format: PixelFormat,
    alpha_format: AlphaFormat,
) {
    let pixel_bytes = bytes_per_pixel(format);
    let width = size.width.max(0) as usize;

    for (y, source_row) in rgba.chunks_exact(width.max(1) * 4).enumerate() {
        let Some(target_row) = data.get_mut(y * row_bytes..y * row_bytes + width * pixel_bytes) else {
            break;
        };

        for (target, source) in target_row.chunks_exact_mut(pixel_bytes).zip(source_row.chunks_exact(4)) {
            let (mut r, mut g, mut b, mut a) = (source[0], source[1], source[2], source[3]);

            if format == PixelFormat::RGB565 {
                // Rounded to five and six bits.
                let scale = |channel: u8, max: u32| ((channel as u32 * max + 127) / 255) as u16;
                let value = (scale(r, 31) << 11) | (scale(g, 63) << 5) | scale(b, 31);
                target.copy_from_slice(&value.to_le_bytes());
                continue;
            }
            if format == PixelFormat::RGB32 {
                target.copy_from_slice(&[r, g, b, 255]);
                continue;
            }

            match alpha_format {
                AlphaFormat::Premul => {}
                AlphaFormat::Opaque => a = 255,
                AlphaFormat::Unpremul => {
                    if a != 0 && a != 255 {
                        (r, g, b) = (unpremultiply(r, a), unpremultiply(g, a), unpremultiply(b, a));
                    }
                }
            }

            if format == PixelFormat::BGRA8888 {
                std::mem::swap(&mut r, &mut b);
            }

            target.copy_from_slice(&[r, g, b, a]);
        }
    }
}

/// Premultiplied RGBA pixels without padding as an image the scene of a
/// frame can paint with.
pub fn to_image(rgba: Vec<u8>, size: PixelSize) -> ImageData {
    to_shared_image(Arc::new(rgba), size)
}

/// [`to_image`] of pixels that stay shared with their owner: nothing is
/// copied. The owner changes its pixels only once it is the only one that
/// holds them (`Arc::make_mut`).
pub fn to_shared_image(rgba: Arc<Vec<u8>>, size: PixelSize) -> ImageData {
    ImageData {
        data: Blob::new(rgba),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::AlphaPremultiplied,
        width: size.width.max(0) as u32,
        height: size.height.max(0) as u32,
    }
}

#[cfg(test)]
mod tests {
    // Not from the Skia backend, whose conversions are Skia's.
    use super::*;

    const SIZE: PixelSize = PixelSize::new(2, 1);

    #[test]
    fn sixteen_bits_a_pixel_are_read_and_written() {
        // Pure red and a grey: 0xF800, and 16, 32 and 16 of 31, 63 and 31.
        let data = [0x00, 0xf8, 0x10, 0x84];
        assert_eq!(2, bytes_per_pixel(PixelFormat::RGB565));

        let rgba = to_premul_rgba(&data, 4, SIZE, PixelFormat::RGB565, AlphaFormat::Opaque);
        assert_eq!(vec![255, 0, 0, 255, 132, 130, 132, 255], rgba);

        // Reading and writing give the same bits, for every value.
        let mut written = [0u8; 4];
        from_premul_rgba(&rgba, &mut written, 4, SIZE, PixelFormat::RGB565, AlphaFormat::Opaque);
        assert_eq!(data, written);
        for value in 0..=u16::MAX {
            let pixel = to_premul_rgba(&value.to_le_bytes(), 2, PixelSize::new(1, 1), PixelFormat::RGB565, AlphaFormat::Premul);
            let mut back = [0u8; 2];
            from_premul_rgba(&pixel, &mut back, 2, PixelSize::new(1, 1), PixelFormat::RGB565, AlphaFormat::Premul);
            assert_eq!(value, u16::from_le_bytes(back));
        }

        // A translucent pixel is written as it is drawn, over black.
        let mut written = [0u8; 2];
        from_premul_rgba(&[128, 0, 0, 128], &mut written, 2, PixelSize::new(1, 1), PixelFormat::RGB565, AlphaFormat::Premul);
        assert_eq!(16 << 11, u16::from_le_bytes(written));
    }

    #[test]
    fn the_alpha_formats_are_converted() {
        let translucent = [200, 100, 50, 128, 0, 0, 0, 0];

        // Not premultiplied: the colors are multiplied by the alpha, and
        // divided again when written.
        let rgba = to_premul_rgba(&translucent, 8, SIZE, PixelFormat::RGBA8888, AlphaFormat::Unpremul);
        assert_eq!(vec![100, 50, 25, 128, 0, 0, 0, 0], rgba);
        let mut written = [9u8; 8];
        from_premul_rgba(&rgba, &mut written, 8, SIZE, PixelFormat::RGBA8888, AlphaFormat::Unpremul);
        assert_eq!([199, 100, 50, 128, 0, 0, 0, 0], written);

        // Opaque: the alpha byte is not read, and written as 255.
        let rgba = to_premul_rgba(&translucent, 8, SIZE, PixelFormat::BGRA8888, AlphaFormat::Opaque);
        assert_eq!(vec![50, 100, 200, 255, 0, 0, 0, 255], rgba);
        let mut written = [9u8; 8];
        from_premul_rgba(&[100, 50, 25, 128, 0, 0, 0, 0], &mut written, 8, SIZE, PixelFormat::BGRA8888, AlphaFormat::Opaque);
        assert_eq!([25, 50, 100, 255, 0, 0, 0, 255], written);

        // Premultiplied: only the order of the channels.
        let rgba = to_premul_rgba(&translucent, 8, SIZE, PixelFormat::BGRA8888, AlphaFormat::Premul);
        assert_eq!(vec![50, 100, 200, 128, 0, 0, 0, 0], rgba);

        // Thirty-two bits without alpha.
        let rgba = to_premul_rgba(&translucent, 8, SIZE, PixelFormat::RGB32, AlphaFormat::Premul);
        assert_eq!(vec![200, 100, 50, 255, 0, 0, 0, 255], rgba);
        let mut written = [9u8; 8];
        from_premul_rgba(&rgba, &mut written, 8, SIZE, PixelFormat::RGB32, AlphaFormat::Premul);
        assert_eq!([200, 100, 50, 255, 0, 0, 0, 255], written);
    }

    #[test]
    fn rows_have_their_padding_and_formats_are_known() {
        // Two rows of a pixel with padding: the padding is not read and
        // not written.
        let size = PixelSize::new(1, 2);
        let rgba = to_premul_rgba(&[1, 2, 3, 4, 77, 77, 5, 6, 7, 8, 77, 77], 6, size, PixelFormat::RGBA8888, AlphaFormat::Premul);
        assert_eq!(vec![1, 2, 3, 4, 5, 6, 7, 8], rgba);
        let mut written = [9u8; 12];
        from_premul_rgba(&rgba, &mut written, 6, size, PixelFormat::RGBA8888, AlphaFormat::Premul);
        assert_eq!([1, 2, 3, 4, 9, 9, 5, 6, 7, 8, 9, 9], written);

        assert!(is_supported(PixelFormat::RGB565) && is_supported(PixelFormat::BGRA8888) && is_supported(PixelFormat::RGBA8888));
        assert!(!is_supported(PixelFormat::RGB32));
        assert_eq!(Some(4), try_bytes_per_pixel(PixelFormat::RGB32));
        assert_eq!(None, try_bytes_per_pixel(ferroui_base::platform::PixelFormats::GRAY8));
    }
}
