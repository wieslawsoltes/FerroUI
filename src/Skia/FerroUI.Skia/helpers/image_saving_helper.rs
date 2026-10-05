use ferroui_base::media::imaging::{BitmapEncoderOptions, CompressionLevel};
use skia_safe::{jpeg_encoder, png_encoder, Image, Pixmap};
use std::io::{self, Write};

fn invalid_input(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn other(message: &str) -> io::Error {
    io::Error::other(message)
}

/// Saves a Skia image to a file.
pub fn save_image_to_file(image: &Image, file_name: &str, options: &BitmapEncoderOptions) -> io::Result<()> {
    let mut stream = std::fs::File::create(file_name)?;
    save_image(image, &mut stream, options)
}

/// Saves a Skia image to a stream.
///
/// Texture-backed images are read back into memory first.
pub fn save_image(image: &Image, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> io::Result<()> {
    // Validate the options before doing any work.
    if let BitmapEncoderOptions::Jpeg(jpeg_options) = options {
        if !(0..=100).contains(&jpeg_options.quality) {
            return Err(invalid_input("Unknown quality"));
        }
    }

    let raster = image
        .make_raster_image(None, skia_safe::image::CachingHint::Allow)
        .ok_or_else(|| other("Could not get image pixels"))?;
    let pixmap = raster.peek_pixels().ok_or_else(|| other("Could not get image pixels"))?;

    save_pixmap(&pixmap, stream, options)
}

/// Encodes a pixmap to a stream.
pub fn save_pixmap(pixmap: &Pixmap, mut stream: &mut dyn Write, options: &BitmapEncoderOptions) -> io::Result<()> {
    let encoded = match options {
        BitmapEncoderOptions::Png(options) => {
            png_encoder::encode(pixmap, &mut stream, &png_options(options.compression_level))
        }
        BitmapEncoderOptions::Jpeg(options) => {
            if !(0..=100).contains(&options.quality) {
                return Err(invalid_input("Unknown quality"));
            }
            jpeg_encoder::encode(pixmap, &mut stream, &jpeg_options(options.quality))
        }
    };

    if encoded {
        Ok(())
    } else {
        Err(other("Could not encode image"))
    }
}

fn png_options(compression_level: CompressionLevel) -> png_encoder::Options {
    let z_lib_level = match compression_level {
        CompressionLevel::Optimal => 6,
        CompressionLevel::Fastest => 1,
        CompressionLevel::NoCompression => 0,
        CompressionLevel::SmallestSize => 9,
    };

    let mut options = png_encoder::Options::default();
    options.filter_flags = png_encoder::FilterFlag::ALL;
    options.z_lib_level = z_lib_level;
    options
}

fn jpeg_options(quality: i32) -> jpeg_encoder::Options {
    jpeg_encoder::Options { quality: quality as u32, ..jpeg_encoder::Options::default() }
}
