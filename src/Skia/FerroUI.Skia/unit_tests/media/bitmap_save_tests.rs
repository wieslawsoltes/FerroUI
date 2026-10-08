//! Port of upstream's `Media/BitmapSaveTests.cs` of the Skia unit tests.
//!
//! `ArgumentOutOfRangeException` thrown by `Save` is an `Err` of kind
//! `InvalidInput` in the port. Two upstream tests have no counterpart
//! because the port's types cannot express their input:
//! `Save_With_Null_Options_Throws` (the options are a reference, never
//! null) and `Save_With_Invalid_Png_CompressionLevel_Throws` (a
//! `CompressionLevel` is a closed enum, `(CompressionLevel)42` cannot be
//! constructed).

use crate::unit_tests::mock_platform_render_interface;
use crate::PlatformRenderInterface;
use ferroui_base::media::imaging::{IBitmap, JpegBitmapEncoderOptions, PngBitmapEncoderOptions, WriteableBitmap};
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::{PixelSize, Vector};
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use skia_safe::{Codec, Color, Data, EncodedImageFormat};
use std::rc::Rc;

#[test]
fn save_with_invalid_jpeg_quality_throws() {
    let _app = start();

    let bitmap = create_bitmap(Color::RED, 16, 16);
    let mut stream = Vec::new();
    let options = JpegBitmapEncoderOptions { quality: -1 };

    let error = bitmap.save(&mut stream, &options.into()).unwrap_err();
    assert_eq!(std::io::ErrorKind::InvalidInput, error.kind());
}

#[test]
fn save_with_png_options_produces_png() {
    let _app = start();

    let bitmap = create_bitmap(Color::RED, 16, 16);
    let mut stream = Vec::new();

    bitmap.save(&mut stream, &PngBitmapEncoderOptions::DEFAULT.into()).unwrap();

    let codec = Codec::from_data(Data::new_copy(&stream)).unwrap();

    assert_eq!(EncodedImageFormat::PNG, codec.encoded_format());
}

#[test]
fn save_with_jpeg_options_produces_jpeg() {
    let _app = start();

    let bitmap = create_bitmap(Color::RED, 16, 16);
    let mut stream = Vec::new();

    bitmap.save(&mut stream, &JpegBitmapEncoderOptions::DEFAULT.into()).unwrap();

    let codec = Codec::from_data(Data::new_copy(&stream)).unwrap();

    assert_eq!(EncodedImageFormat::JPEG, codec.encoded_format());
}

fn create_bitmap(color: Color, width: i32, height: i32) -> WriteableBitmap {
    let pixel = ((color.a() as u32) << 24) | ((color.r() as u32) << 16) | ((color.g() as u32) << 8) | color.b() as u32;

    let data = vec![pixel as i32; (width * height) as usize];

    create_bitmap_from_data(width, height, &data)
}

fn create_bitmap_from_data(width: i32, height: i32, data: &[i32]) -> WriteableBitmap {
    let bitmap = WriteableBitmap::new(
        PixelSize::new(width, height),
        Vector::new(96.0, 96.0),
        Some(PixelFormat::BGRA8888),
        Some(AlphaFormat::Premul),
    );

    let fb = bitmap.lock();
    let row_bytes = fb.row_bytes() as usize;

    fb.with_data(&mut |pixels| {
        for y in 0..height as usize {
            let row = &data[y * width as usize..(y + 1) * width as usize];
            for (x, value) in row.iter().enumerate() {
                let offset = y * row_bytes + x * 4;
                pixels[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
            }
        }
    });

    fb.dispose();

    bitmap
}

fn start() -> UnitTestApplicationScope {
    UnitTestApplication::start(
        mock_platform_render_interface().with_render_interface(Rc::new(PlatformRenderInterface::default())),
    )
}
