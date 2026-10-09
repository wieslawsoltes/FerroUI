//! The codecs of the two backends against each other: the same encoded
//! data decoded by both, and what one encodes decoded by the other.
//!
//! Run with `--nocapture` to see the table of the design document.

use crate::{compare, Backend, Difference, Pixels};
use ferroui_base::media::imaging::{
    BitmapEncoderOptions, BitmapInterpolationMode, JpegBitmapEncoderOptions, PngBitmapEncoderOptions,
};
use ferroui_base::platform::{AlphaFormat, IBitmapImpl, PixelFormat, SharedBitmapImpl};
use ferroui_base::{PixelSize, Vector};
use std::sync::Arc;

const DPI: Vector = Vector::new(96.0, 96.0);

fn backends() -> (Backend, Backend) {
    (Backend::skia(), Backend::vello(ferroui_vello::VelloRenderingMode::Cpu))
}

/// The pixels of a bitmap as premultiplied RGBA. A bitmap that was decoded
/// from an image without alpha is opaque in both backends: its fourth byte
/// is read as 255.
fn pixels_of(bitmap: &dyn IBitmapImpl) -> Pixels {
    let framebuffer = bitmap.as_readable_bitmap().expect("a readable bitmap").lock();
    let size = framebuffer.size();
    let (width, height) = (size.width as usize, size.height as usize);
    let row_bytes = framebuffer.row_bytes() as usize;
    let (format, alpha_format) = (framebuffer.format(), framebuffer.alpha_format());
    assert!(format == PixelFormat::RGBA8888 || format == PixelFormat::BGRA8888, "an unexpected format: {format:?}");
    assert!(alpha_format != AlphaFormat::Unpremul);

    let mut rgba = Vec::with_capacity(width * height * 4);
    framebuffer.with_data(&mut |data| {
        for y in 0..height {
            for pixel in data[y * row_bytes..y * row_bytes + width * 4].chunks_exact(4) {
                let alpha = if alpha_format == AlphaFormat::Opaque { 255 } else { pixel[3] };
                if format == PixelFormat::BGRA8888 {
                    rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], alpha]);
                } else {
                    rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], alpha]);
                }
            }
        }
    });
    framebuffer.dispose();

    Pixels { width, height, rgba }
}

/// The alpha format a backend reports for a bitmap it decodes.
fn alpha_format_of(backend: &Backend, data: &[u8]) -> Option<AlphaFormat> {
    backend.interface.load_bitmap(&mut &data[..]).ok()?.as_readable_bitmap()?.alpha_format()
}

fn decode(backend: &Backend, data: &[u8]) -> Option<Pixels> {
    backend.interface.load_bitmap(&mut &data[..]).ok().map(|bitmap| pixels_of(&*bitmap))
}

/// Decodes data with both backends and compares the pixels. `None` when
/// one of them does not decode it.
fn decoded_by_both(data: &[u8]) -> Option<Difference> {
    let (skia, vello) = backends();
    match (decode(&skia, data), decode(&vello, data)) {
        (Some(reference), Some(tested)) => {
            assert_eq!((reference.width, reference.height), (tested.width, tested.height));
            Some(compare(&reference, &tested))
        }
        (reference, tested) => {
            println!("decoded by Skia: {}, by Vello: {}", reference.is_some(), tested.is_some());
            None
        }
    }
}

fn row(what: &str, difference: Difference) {
    println!("| {what} | {:.3} % | {} | {:.3} |", difference.share, difference.largest, difference.mean);
}

fn header() {
    println!("| Data | Pixels beyond the tolerance of {} | Largest difference | Mean difference |", crate::TOLERANCE);
    println!("|---|---|---|---|");
}

/// A picture of 64 by 48 pixels with ramps, a block and translucent
/// pixels, as a bitmap of a backend.
fn picture(backend: &Backend, translucent: bool) -> Arc<SharedBitmapImpl> {
    let mut data = Vec::new();
    for y in 0..48u32 {
        for x in 0..64u32 {
            let blue = if (16..48).contains(&x) && (12..36).contains(&y) { 220 } else { 30 };
            let alpha = if translucent && x >= 32 { 255 - (y * 5) } else { 255 };
            let premultiply = |channel: u32| ((channel * alpha + 127) / 255) as u8;
            data.extend_from_slice(&[premultiply(x * 4), premultiply(y * 5), premultiply(blue), alpha as u8]);
        }
    }

    backend.interface.load_bitmap_from_pixels(
        PixelFormat::RGBA8888,
        AlphaFormat::Premul,
        &data,
        PixelSize::new(64, 48),
        DPI,
        64 * 4,
    )
}

fn encode(bitmap: &dyn IBitmapImpl, options: BitmapEncoderOptions) -> Vec<u8> {
    let mut encoded = Vec::new();
    bitmap.save(&mut encoded, &options).expect("an encoded bitmap");
    encoded
}

fn png() -> BitmapEncoderOptions {
    PngBitmapEncoderOptions::DEFAULT.into()
}

fn jpeg(quality: i32) -> BitmapEncoderOptions {
    JpegBitmapEncoderOptions { quality }.into()
}

/// PNG: what each backend encodes is decoded by the other to the pixels
/// that were encoded.
#[test]
fn png_of_one_backend_is_decoded_by_the_other() {
    let (skia, vello) = backends();
    header();

    for translucent in [false, true] {
        let name = if translucent { "translucent" } else { "opaque" };
        let (original_skia, original_vello) = (picture(&skia, translucent), picture(&vello, translucent));
        assert_eq!(pixels_of(&*original_skia), pixels_of(&*original_vello));
        let original = pixels_of(&*original_skia);

        let by_skia = encode(&*original_skia, png());
        let by_vello = encode(&*original_vello, png());

        let pairs = [
            ("Skia", "Vello", decode(&vello, &by_skia)),
            ("Vello", "Skia", decode(&skia, &by_vello)),
            ("Vello", "Vello", decode(&vello, &by_vello)),
            ("Skia", "Skia", decode(&skia, &by_skia)),
        ];
        for (encoder, decoder, decoded) in pairs {
            let difference = compare(&original, &decoded.expect("a decoded PNG"));
            row(&format!("PNG, {name}, encoded by {encoder}, decoded by {decoder}, against the picture"), difference);
            // A PNG holds colors that are not premultiplied: a translucent
            // pixel comes back within a step of a channel.
            assert!(difference.largest <= if translucent { 2 } else { 0 }, "{encoder} to {decoder}: {difference:?}");
        }
    }
}

/// JPEG: what each backend encodes at a quality is decoded by both, and is
/// as far from the picture as what the other encodes.
#[test]
fn jpeg_of_one_backend_is_decoded_by_the_other() {
    let (skia, vello) = backends();
    header();

    let (original_skia, original_vello) = (picture(&skia, false), picture(&vello, false));
    let original = pixels_of(&*original_skia);

    for quality in [100, 75, 30] {
        let by_skia = encode(&*original_skia, jpeg(quality));
        let by_vello = encode(&*original_vello, jpeg(quality));
        println!("| JPEG of quality {quality}: {} bytes by Skia, {} bytes by Vello | | | |", by_skia.len(), by_vello.len());

        // The same file by both decoders.
        for (encoder, data) in [("Skia", &by_skia), ("Vello", &by_vello)] {
            let difference = decoded_by_both(data).expect("a JPEG that both decode");
            row(&format!("JPEG of quality {quality} encoded by {encoder}: decoded by Vello against decoded by Skia"), difference);
            assert!(difference.share < 0.5 && difference.mean < 2.0, "{difference:?}");
        }

        // Each encoder against the picture, by the decoder of Skia.
        let from_skia = compare(&original, &decode(&skia, &by_skia).expect("a decoded JPEG"));
        let from_vello = compare(&original, &decode(&skia, &by_vello).expect("a decoded JPEG"));
        row(&format!("JPEG of quality {quality} encoded by Skia, against the picture"), from_skia);
        row(&format!("JPEG of quality {quality} encoded by Vello, against the picture"), from_vello);
        // The two encoders lose about as much, and write files of about
        // one size.
        assert!(from_vello.mean < from_skia.mean * 1.5 + 0.5, "{from_vello:?} against {from_skia:?}");
        assert!(by_vello.len() < by_skia.len() * 2 && by_skia.len() < by_vello.len() * 2);
    }

    // A quality out of range is refused by both, the same way.
    for backend in [&skia, &vello] {
        let bitmap = picture(backend, false);
        let mut encoded = Vec::new();
        let error = bitmap.save(&mut encoded, &jpeg(101)).expect_err("a refused quality");
        assert_eq!(std::io::ErrorKind::InvalidInput, error.kind(), "{}", backend.name);
    }
}

/// The photographs of the sample application, by both decoders.
#[test]
fn the_pictures_of_the_catalog_are_decoded_alike() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/ControlCatalog/Assets");
    let (mut files, mut worst_share, mut worst_mean, mut worst_largest) = (0, 0.0f64, 0.0f64, 0u8);
    header();

    for folder in ["CurvedHeader", "ModernApp"] {
        let mut paths: Vec<_> = std::fs::read_dir(directory.join(folder))
            .expect("the assets of the catalog")
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "jpg"))
            .collect();
        paths.sort();

        for path in paths {
            let data = std::fs::read(&path).unwrap();
            let difference = decoded_by_both(&data).unwrap_or_else(|| panic!("{} is decoded by both", path.display()));
            row(&format!("`{folder}/{}`", path.file_name().unwrap().to_string_lossy()), difference);

            files += 1;
            worst_share = worst_share.max(difference.share);
            worst_mean = worst_mean.max(difference.mean);
            worst_largest = worst_largest.max(difference.largest);
        }
    }

    println!("| the worst of {files} files | {worst_share:.3} % | {worst_largest} | {worst_mean:.3} |");
    assert!(files >= 10);
    // Two decoders of JPEG differ in how they round the inverse transform
    // and enlarge the color difference channels: by a few steps, nowhere
    // by the tolerance on more than a pixel in a thousand.
    assert!(worst_share < 0.1 && worst_mean < 2.0, "{worst_share} {worst_mean}");
}

/// The forty bytes of the header of a bitmap.
fn bitmap_header(width: i32, height: i32, bits_per_pixel: u16, colors: u32) -> Vec<u8> {
    let mut header = Vec::new();
    header.extend_from_slice(&40u32.to_le_bytes());
    header.extend_from_slice(&width.to_le_bytes());
    header.extend_from_slice(&height.to_le_bytes());
    header.extend_from_slice(&1u16.to_le_bytes());
    header.extend_from_slice(&bits_per_pixel.to_le_bytes());
    header.extend_from_slice(&[0; 8]);
    header.extend_from_slice(&2835u32.to_le_bytes());
    header.extend_from_slice(&2835u32.to_le_bytes());
    header.extend_from_slice(&colors.to_le_bytes());
    header.extend_from_slice(&[0; 4]);
    header
}

fn bitmap_file(header: &[u8], rest: &[u8], palette_bytes: usize) -> Vec<u8> {
    let mut file = Vec::new();
    file.extend_from_slice(b"BM");
    file.extend_from_slice(&((14 + header.len() + rest.len()) as u32).to_le_bytes());
    file.extend_from_slice(&[0; 4]);
    file.extend_from_slice(&((14 + header.len() + palette_bytes) as u32).to_le_bytes());
    file.extend_from_slice(header);
    file.extend_from_slice(rest);
    file
}

/// Rows of 24 bits a pixel, padded to four bytes, of a picture with a
/// color for every pixel.
fn rows_24(width: usize, height: usize) -> Vec<u8> {
    let mut rows = Vec::new();
    for y in 0..height {
        for x in 0..width {
            rows.extend_from_slice(&[(x * 37 + y * 11) as u8, (x * 5 + y * 59) as u8, (x * 83 + y * 3) as u8]);
        }
        rows.resize(rows.len().next_multiple_of(4), 0);
    }
    rows
}

fn ico(images: &[(u8, u8, Vec<u8>)]) -> Vec<u8> {
    let mut file = vec![0, 0, 1, 0];
    file.extend_from_slice(&(images.len() as u16).to_le_bytes());

    let mut offset = 6 + 16 * images.len();
    for (width, height, data) in images {
        file.extend_from_slice(&[*width, *height, 0, 0, 1, 0, 32, 0]);
        file.extend_from_slice(&(data.len() as u32).to_le_bytes());
        file.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += data.len();
    }
    for (_, _, data) in images {
        file.extend_from_slice(data);
    }
    file
}

/// GIF, BMP and ICO: the same files by both decoders, to the last digit of
/// a color.
#[test]
fn gif_bmp_and_ico_are_decoded_alike() {
    header();
    let (skia, vello) = backends();
    let check = |what: &str, data: &[u8]| {
        let difference = decoded_by_both(data).unwrap_or_else(|| panic!("{what} is decoded by both"));
        row(what, difference);
        assert_eq!(0, difference.largest, "{what}");
        // Whether the image has alpha is told alike.
        assert_eq!(alpha_format_of(&skia, data), alpha_format_of(&vello, data), "{what}");
    };

    // GIF: two frames, the first smaller than the image and with a
    // transparent color.
    let mut gif_file = Vec::new();
    {
        let palette = [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
        let mut encoder = gif::Encoder::new(&mut gif_file, 9, 7, &palette).unwrap();
        let indices: Vec<u8> = (0..8 * 6).map(|index| ((index * 7) % 4) as u8).collect();
        encoder
            .write_frame(&gif::Frame {
                left: 1,
                top: 1,
                width: 8,
                height: 6,
                transparent: Some(3),
                buffer: std::borrow::Cow::Borrowed(&indices),
                ..gif::Frame::default()
            })
            .unwrap();
        encoder
            .write_frame(&gif::Frame { width: 9, height: 7, buffer: std::borrow::Cow::Owned(vec![1; 63]), ..gif::Frame::default() })
            .unwrap();
    }
    check("GIF, the first of two frames, with a transparent color", &gif_file);

    // BMP: 24 bits a pixel from the bottom up and from the top down, and
    // eight bits with a palette.
    check("BMP, 24 bits a pixel", &bitmap_file(&bitmap_header(7, 5, 24, 0), &rows_24(7, 5), 0));
    check("BMP, 24 bits a pixel, rows from the top down", &bitmap_file(&bitmap_header(7, -5, 24, 0), &rows_24(7, 5), 0));
    let mut indexed = vec![0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0, 0, 40, 40, 40, 0];
    for y in 0..5usize {
        indexed.extend((0..8).map(|x| ((x + y) % 4) as u8));
    }
    check("BMP, 8 bits a pixel with a palette", &bitmap_file(&bitmap_header(8, 5, 8, 4), &indexed, 16));

    // ICO: a bitmap of 24 bits with its mask, a bitmap of 32 bits with its
    // alpha, an embedded PNG, and the largest of three images.
    let mut with_mask = bitmap_header(7, 10, 24, 0);
    with_mask.extend_from_slice(&rows_24(7, 5));
    for y in 0..5u8 {
        with_mask.extend_from_slice(&[0b1010_0000 >> y, 0, 0, 0]);
    }
    check("ICO, 24 bits a pixel with a mask", &ico(&[(7, 5, with_mask.clone())]));

    let mut with_alpha = bitmap_header(4, 6, 32, 0);
    for index in 0..12u8 {
        with_alpha.extend_from_slice(&[index * 20, 255 - index * 20, 77, 255 - index * 21]);
    }
    with_alpha.extend_from_slice(&[0; 12]);
    check("ICO, 32 bits a pixel with alpha", &ico(&[(4, 3, with_alpha.clone())]));

    let embedded = encode(&*picture(&skia, true), png());
    check("ICO, an embedded PNG", &ico(&[(64, 48, embedded.clone())]));
    check(
        "ICO, the largest of three images",
        &ico(&[(4, 3, with_alpha), (64, 48, embedded), (7, 5, with_mask)]),
    );
}

/// What neither backend decodes in this workspace, and what only Skia
/// does: the formats that fail to load in the Vello backend are listed in
/// the design document.
#[test]
fn formats_outside_the_list_fail_to_load() {
    let (skia, vello) = backends();

    for data in [&b"not an image"[..], &[][..], &[1, 2, 3, 4][..]] {
        assert!(decode(&skia, data).is_none());
        assert!(decode(&vello, data).is_none());
    }

    // A wireless bitmap of 10 by 2 pixels: both load it. The Skia backend
    // decodes it to a bitmap of gray, which it draws and cannot lock; the
    // two are compared as they are drawn.
    let wbmp = [0u8, 0, 10, 2, 0b1010_0000, 0b0100_0000, 0b0000_0000, 0b1100_0000];
    let drawn = |backend: &Backend| {
        let bitmap = backend.interface.load_bitmap(&mut &wbmp[..]).expect("a decoded wireless bitmap");
        assert_eq!(PixelSize::new(10, 2), bitmap.pixel_size());
        crate::render(backend, PixelSize::new(10, 2), &|_, context| {
            context.draw_bitmap(
                &*bitmap,
                1.0,
                ferroui_base::Rect::new(0.0, 0.0, 10.0, 2.0),
                ferroui_base::Rect::new(0.0, 0.0, 10.0, 2.0),
            );
        })
    };
    let difference = compare(&drawn(&skia), &drawn(&vello));
    println!("WBMP, drawn: {difference:?}");
    assert_eq!(0, difference.largest);
}

/// A JPEG decoded to a width in each interpolation mode, and a bitmap
/// resized, by both backends.
#[test]
fn a_bitmap_is_decoded_to_a_size_and_resized_alike() {
    let (skia, vello) = backends();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/ControlCatalog/Assets/ModernApp/dest_alps.jpg");
    let data = std::fs::read(path).unwrap();
    header();

    let natural = skia.interface.load_bitmap(&mut &data[..]).unwrap().pixel_size();

    for mode in [
        BitmapInterpolationMode::None,
        BitmapInterpolationMode::LowQuality,
        BitmapInterpolationMode::MediumQuality,
        BitmapInterpolationMode::HighQuality,
    ] {
        for (what, width) in [("reduced", natural.width * 3 / 10 + 1), ("enlarged", natural.width * 3 / 2 + 1)] {
            let by_skia = skia.interface.load_bitmap_to_width(&mut &data[..], width, mode).unwrap();
            let by_vello = vello.interface.load_bitmap_to_width(&mut &data[..], width, mode).unwrap();
            assert_eq!(by_skia.pixel_size(), by_vello.pixel_size(), "{mode:?} {what}");

            let difference = compare(&pixels_of(&*by_skia), &pixels_of(&*by_vello));
            row(&format!("a JPEG of {} by {} decoded to a width of {width} ({what}), {mode:?}", natural.width, natural.height), difference);
            // The nearest pixel of a photograph can be the one or the other
            // where the middle of a pixel falls on an edge: every other
            // mode agrees within the tolerance on nearly every pixel.
            if mode != BitmapInterpolationMode::None {
                assert!(difference.share < 1.0 && difference.mean < 4.0, "{mode:?} {what}: {difference:?}");
            }
        }

        let (source_skia, source_vello) = (picture(&skia, true), picture(&vello, true));
        for (what, size) in [("reduced", PixelSize::new(23, 17)), ("enlarged", PixelSize::new(150, 100))] {
            let by_skia = skia.interface.resize_bitmap(&*source_skia, size, mode);
            let by_vello = vello.interface.resize_bitmap(&*source_vello, size, mode);
            let difference = compare(&pixels_of(&*by_skia), &pixels_of(&*by_vello));
            row(&format!("a bitmap of 64 by 48 resized to {} by {} ({what}), {mode:?}", size.width, size.height), difference);
            if mode != BitmapInterpolationMode::None {
                assert!(difference.share < 1.0 && difference.mean < 4.0, "{mode:?} {what}: {difference:?}");
            }
        }
    }
}
