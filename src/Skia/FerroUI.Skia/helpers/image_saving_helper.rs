use ferroui_base::media::imaging::{BitmapEncoderOptions, CompressionLevel, PngBitmapEncoderOptions};
use skia_safe::images::{self, BitDepth};
use skia_safe::{jpeg_encoder, png_encoder, ColorSpace, ISize, Image, Matrix, Picture, Pixmap};
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

/// Saves a picture, drawn at `scale`, to a file in the PNG format.
// This method is here mostly for debugging purposes
#[allow(dead_code)]
pub(crate) fn save_picture(picture: &Picture, scale: f32, path: &str) -> io::Result<()> {
    let cull_rect = picture.cull_rect();
    let snapshot_size =
        ISize::new((cull_rect.width() * scale).ceil() as i32, (cull_rect.height() * scale).ceil() as i32);
    let snap = images::deferred_from_picture(
        picture.clone(),
        snapshot_size,
        Some(&Matrix::scale((scale, scale))),
        None,
        BitDepth::U8,
        ColorSpace::new_srgb(),
        None,
    )
    .ok_or_else(|| invalid_input("Could not create an image of the picture"))?;
    save_image_to_file(&snap, path, &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT))
}

#[cfg(test)]
mod tests {
    // Not from upstream, which has no test of the method.
    use super::*;
    use skia_safe::{Color, Data, PictureRecorder, Rect};

    #[test]
    fn save_picture_writes_the_picture_at_the_scale() {
        let mut recorder = PictureRecorder::new();
        let canvas = recorder.begin_recording(Rect::new(0.0, 0.0, 10.0, 4.5), false);
        canvas.clear(Color::RED);
        let picture = recorder.finish_recording_as_picture(None).expect("a recorded picture");

        let path = std::env::temp_dir().join(format!("ferroui-save-picture-{}.png", std::process::id()));
        let saved = save_picture(&picture, 2.0, path.to_str().expect("a path that is text"));
        let bytes = std::fs::read(&path);
        let _ = std::fs::remove_file(&path);
        saved.expect("the picture is saved");

        // Encoded as PNG, with the size of the picture at the scale.
        let bytes = bytes.expect("the file is written");
        assert_eq!(&[0x89, b'P', b'N', b'G'], &bytes[..4]);
        let image = Image::from_encoded(Data::new_copy(&bytes)).expect("an image that decodes");
        assert_eq!((20, 9), (image.width(), image.height()));
        let raster = image.make_raster_image(None, skia_safe::image::CachingHint::Allow).expect("the pixels");
        assert_eq!(Color::RED, raster.peek_pixels().expect("the pixels").get_color((10, 4)));
    }
}
