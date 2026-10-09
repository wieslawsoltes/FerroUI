use ferroui_base::media::imaging::{BitmapEncoderOptions, CompressionLevel};
use peniko::ImageData;
use std::io::{self, Write};

pub use crate::helpers::image_decoding_helper::decode_image;
pub(crate) use crate::helpers::image_decoding_helper::load_error;

fn invalid_input(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

/// Saves an image of premultiplied RGBA pixels to a file.
pub fn save_image_to_file(image: &ImageData, file_name: &str, options: &BitmapEncoderOptions) -> io::Result<()> {
    let mut stream = std::fs::File::create(file_name)?;
    save_image(image, &mut stream, options)
}

/// Saves an image of premultiplied RGBA pixels to a stream, in one of the
/// two formats of the contract: PNG with the `png` crate, JPEG with the
/// `jpeg-encoder` crate.
pub fn save_image(image: &ImageData, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> io::Result<()> {
    match options {
        BitmapEncoderOptions::Png(options) => save_png(image, stream, options.compression_level),
        BitmapEncoderOptions::Jpeg(options) => {
            // Validate the options before doing any work.
            if !(0..=100).contains(&options.quality) {
                return Err(invalid_input("Unknown quality"));
            }
            save_jpeg(image, stream, options.quality as u8)
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

/// Encodes an image as a baseline JPEG, as the encoder of Skia does with
/// its default options: the colors as they are premultiplied (a JPEG has no
/// alpha: what is translucent is written as it is drawn over black) and the
/// two color difference channels at half the resolution in both directions,
/// each of their samples the mean of four pixels. The Huffman tables are
/// the standard ones: with tables made for the image, which the encoder of
/// Skia writes and which make a file about a third smaller, what this
/// encoder wrote was decoded as black by the decoder of this backend (which
/// of the two crates is at fault was not examined).
/// A quality of 0 is the lowest quality there is, 1, as in libjpeg.
fn save_jpeg(image: &ImageData, stream: &mut dyn Write, quality: u8) -> io::Result<()> {
    let (Ok(width), Ok(height)) = (u16::try_from(image.width), u16::try_from(image.height)) else {
        return Err(io::Error::other("Could not encode image: a JPEG is at most 65535 pixels wide and high"));
    };
    if width == 0 || height == 0 {
        return Err(io::Error::other("Could not encode image"));
    }

    let mut encoder = jpeg_encoder::Encoder::new(stream, quality.max(1));
    encoder.set_sampling_factor(jpeg_encoder::SamplingFactor::F_2_2);
    encoder.set_chroma_subsampling_method(jpeg_encoder::ChromaSubsamplingMethod::Average);
    // The alpha of the pixels is passed over by the encoder.
    encoder.encode(image.data.data(), width, height, jpeg_encoder::ColorType::Rgba).map_err(io::Error::other)
}
