use ferroui_base::media::imaging::{BitmapEncoderOptions, CompressionLevel};
use peniko::ImageData;
use std::io::{self, Read, Write};

fn invalid_input(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

/// Saves an image of premultiplied RGBA pixels to a stream.
///
/// PNG is written with the `png` crate. JPEG is not built yet: it fails with
/// the stage it belongs to (design document, stage 6).
pub fn save_image(image: &ImageData, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> io::Result<()> {
    match options {
        BitmapEncoderOptions::Png(options) => save_png(image, stream, options.compression_level),
        BitmapEncoderOptions::Jpeg(options) => {
            if !(0..=100).contains(&options.quality) {
                return Err(invalid_input("Unknown quality"));
            }
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "The Vello backend does not encode JPEG yet: stage 6 of docs/porting/vello-backend.md",
            ))
        }
    }
}

fn save_png(image: &ImageData, stream: &mut dyn Write, compression_level: CompressionLevel) -> io::Result<()> {
    // A PNG holds colors that are not premultiplied.
    let mut pixels = image.data.data().to_vec();
    for pixel in pixels.chunks_exact_mut(4) {
        let a = pixel[3] as u32;
        if a != 0 && a != 255 {
            for channel in &mut pixel[..3] {
                *channel = ((*channel as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }

    let mut encoder = png::Encoder::new(stream, image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(match compression_level {
        CompressionLevel::NoCompression => png::Compression::NoCompression,
        CompressionLevel::Fastest => png::Compression::Fast,
        CompressionLevel::Optimal => png::Compression::Balanced,
        CompressionLevel::SmallestSize => png::Compression::High,
    });

    let mut writer = encoder.write_header().map_err(io::Error::other)?;
    writer.write_image_data(&pixels).map_err(io::Error::other)?;
    writer.finish().map_err(io::Error::other)
}

pub(crate) fn load_error() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Unable to load bitmap from provided data")
}

/// Decodes an encoded image at its natural size into premultiplied RGBA
/// pixels without padding, with its width and height.
///
/// PNG is decoded with the `png` crate. The other formats the Skia backend
/// decodes (JPEG, GIF, WebP, BMP, ICO) are not built yet: data in them fails
/// to load, as data in no format does (design document, stage 6).
pub fn decode_image(stream: &mut dyn Read) -> io::Result<(Vec<u8>, u32, u32)> {
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes)?;

    let mut decoder = png::Decoder::new(io::Cursor::new(bytes));
    // Palettes, low bit depths and transparency chunks are expanded and
    // sixteen bits a channel are reduced to eight.
    decoder.set_transformations(png::Transformations::normalize_to_color8());

    let mut reader = decoder.read_info().map_err(|_| load_error())?;
    let buffer_size = reader.output_buffer_size().ok_or_else(load_error)?;
    let mut buffer = vec![0u8; buffer_size];
    let info = reader.next_frame(&mut buffer).map_err(|_| load_error())?;
    let decoded = &buffer[..info.buffer_size()];

    let premultiply = |c: u8, a: u8| ((c as u32 * a as u32 + 127) / 255) as u8;
    let pixel_count = info.width as usize * info.height as usize;
    let mut rgba = Vec::with_capacity(pixel_count * 4);

    match info.color_type {
        png::ColorType::Rgba => {
            for p in decoded.chunks_exact(4) {
                rgba.extend_from_slice(&[premultiply(p[0], p[3]), premultiply(p[1], p[3]), premultiply(p[2], p[3]), p[3]]);
            }
        }
        png::ColorType::Rgb => {
            for p in decoded.chunks_exact(3) {
                rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for p in decoded.chunks_exact(2) {
                let gray = premultiply(p[0], p[1]);
                rgba.extend_from_slice(&[gray, gray, gray, p[1]]);
            }
        }
        png::ColorType::Grayscale => {
            for p in decoded {
                rgba.extend_from_slice(&[*p, *p, *p, 255]);
            }
        }
        png::ColorType::Indexed => return Err(load_error()),
    }

    if rgba.len() != pixel_count * 4 {
        return Err(load_error());
    }

    Ok((rgba, info.width, info.height))
}
