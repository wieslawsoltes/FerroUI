//! Functional backend contracts: native text appearance is deliberately not compared to Skia pixels.
use super::*;
use ferroui_base::media::{
    immutable::{ImmutablePen, ImmutableSolidColorBrush},
    Colors, IntersectionResult,
};
use ferroui_base::{Matrix, RoundedRect};

#[test]
fn geometry_intersection_distinguishes_containment_overlap_and_empty() {
    let outer = Geometry::rectangle(Rect::new(0., 0., 40., 40.));
    let inner = Geometry::rectangle(Rect::new(10., 10., 10., 10.));
    let overlap = Geometry::rectangle(Rect::new(30., 30., 20., 20.));
    let outside = Geometry::rectangle(Rect::new(60., 60., 10., 10.));
    // Preserve Ferro/Skia's established result orientation: the *argument*
    // is FullyInside when the receiver contains it.
    assert_eq!(
        outer.get_fill_intersection_result(&*inner),
        IntersectionResult::FullyInside
    );
    assert_eq!(
        inner.get_fill_intersection_result(&*outer),
        IntersectionResult::FullyContains
    );
    assert_eq!(
        outer.get_fill_intersection_result(&*overlap),
        IntersectionResult::Intersects
    );
    assert_eq!(
        outer.get_fill_intersection_result(&*outside),
        IntersectionResult::Empty
    );
    assert_eq!(
        outer.get_fill_intersection_result(&*outer),
        IntersectionResult::FullyInside
    );
}

#[test]
fn geometry_bounds_include_fill_only_curves_beyond_the_stroke() {
    let mut builder = Builder::new(None);
    builder.begin_figure(Point::new(10., 10.), true);
    builder.line_to(Point::new(20., 10.), true);
    builder.quadratic_bezier_to(Point::new(80., 60.), Point::new(20., 20.), false);
    builder.end_figure(true);
    let geometry = builder.to_geometry();
    assert!(geometry.fill_contains(Point::new(30., 20.)));
    assert!(geometry.bounds().right() > 40.);
    let pen = ImmutablePen::from_uint32(0xffffffff, 2.);
    assert!(geometry.get_render_bounds(Some(&pen)).right() > 40.);
}

#[test]
fn closing_a_broken_stroke_returns_to_the_original_figure_start() {
    let mut builder = Builder::new(None);
    builder.begin_figure(Point::new(10., 10.), true);
    builder.line_to(Point::new(30., 10.), true);
    builder.line_to(Point::new(30., 30.), false);
    builder.line_to(Point::new(10., 30.), true);
    builder.end_figure(true);
    let geometry = builder.to_geometry();
    let pen = ImmutablePen::from_uint32(0xffffffff, 2.);
    assert!(geometry.fill_contains(Point::new(20., 20.)));
    assert!(geometry.stroke_contains(Some(&pen), Point::new(10., 20.)));
    assert!(!geometry.stroke_contains(Some(&pen), Point::new(30., 20.)));
    assert!(!geometry.stroke_contains(Some(&pen), Point::new(20., 20.)));
}

#[test]
fn asymmetric_rounded_corners_keep_radii_that_fit_between_neighbors() {
    let bitmap = Bitmap::new(PixelSize::new(100, 100), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(true, None);
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::WHITE)),
        None,
        RoundedRect::new(
            Rect::new(0., 0., 100., 100.),
            Vector::new(80., 20.),
            Vector::new(10., 20.),
            Vector::ZERO,
            Vector::ZERO,
        ),
        &Default::default(),
    );
    draw.dispose();
    let pixels = bitmap.pixels();
    assert_eq!(
        pixels[(100 + 40) * 4 + 3],
        0,
        "the large corner must not be clamped to 50"
    );
    assert_eq!(pixels[(10 * 100 + 80) * 4 + 3], 255);
}

#[test]
fn stroke_only_geometry_does_not_create_an_implicit_fill_clip() {
    let geometry =
        QuartzRenderInterface.create_line_geometry(Point::new(2., 2.), Point::new(14., 14.));
    assert!(geometry.try_get_path_data().unwrap().fill.is_none());
    for transform in [Matrix::IDENTITY, Matrix::new(0., 0., 0., 0., 0., 0.)] {
        let bitmap = Bitmap::new(PixelSize::new(16, 16), Vector::new(96., 96.));
        let mut draw = bitmap.drawing(true, None);
        draw.set_transform(transform);
        draw.push_geometry_clip(&*geometry);
        draw.set_transform(Matrix::IDENTITY);
        draw.draw_rectangle(
            Some(&ImmutableSolidColorBrush::new(Colors::WHITE)),
            None,
            RoundedRect::from_rect(Rect::new(0., 0., 16., 16.)),
            &Default::default(),
        );
        draw.pop_geometry_clip();
        draw.dispose();
        assert!(bitmap.pixels().iter().all(|p| *p == 255));
    }
}

#[test]
fn bitmap_native_formats_and_alpha_preserve_exact_writable_lock_bytes() {
    for format in [
        PixelFormat::BGRA8888,
        PixelFormat::RGBA8888,
        PixelFormat::RGB565,
    ] {
        assert!(
            QuartzRenderInterface.is_supported_bitmap_pixel_format(format),
            "{format}"
        );
        for alpha in [
            AlphaFormat::Premul,
            AlphaFormat::Unpremul,
            AlphaFormat::Opaque,
        ] {
            let bitmap = QuartzRenderInterface.create_writeable_bitmap(
                PixelSize::new(2, 1),
                Vector::new(144., 144.),
                format,
                alpha,
            );
            let lock = bitmap.lock();
            assert_eq!(lock.format(), format);
            assert_eq!(
                lock.alpha_format(),
                if format.has_alpha() {
                    alpha
                } else {
                    AlphaFormat::Opaque
                }
            );
            assert_eq!(lock.dpi(), Vector::new(144., 144.));
            // Low alpha retains straight RGB precisely in writable memory,
            // even when its premultiplied GPU representation rounds to zero.
            let expected = if format == PixelFormat::RGB565 {
                vec![0, 248, 224, 7]
            } else {
                vec![1, 2, 3, 1, 17, 33, 65, 129]
            };
            lock.with_data(&mut |bytes| bytes[..expected.len()].copy_from_slice(&expected));
            lock.dispose();
            let lock = bitmap.lock();
            lock.with_data(&mut |bytes| assert_eq!(&bytes[..expected.len()], expected.as_slice()));
            lock.dispose();
            let target = Bitmap::new(PixelSize::new(2, 1), Vector::new(96., 96.));
            let mut draw = target.drawing(false, None);
            draw.draw_bitmap(
                &*bitmap,
                1.,
                Rect::new(0., 0., 2., 1.),
                Rect::new(0., 0., 2., 1.),
            );
            draw.dispose();
            let pixels = target.pixels();
            assert!(pixels[7] > 0);
            if format == PixelFormat::RGB565 {
                assert_eq!(pixels, vec![0, 0, 255, 255, 0, 255, 0, 255]);
            }
        }
    }
}

#[test]
fn bitmap_unchanged_lock_does_not_change_version_and_modified_lock_commits_once() {
    let bitmap = Bitmap::new(PixelSize::new(1, 1), Vector::new(96., 96.));
    let version = bitmap.version();
    let lock = bitmap.lock();
    lock.with_data(&mut |p| assert_eq!(p, &[0; 4]));
    lock.dispose();
    assert_eq!(bitmap.version(), version);
    let lock = bitmap.lock();
    lock.with_data(&mut |p| p.copy_from_slice(&[0, 0, 255, 255]));
    lock.dispose();
    lock.dispose();
    assert_eq!(bitmap.version(), version + 1);
    assert_eq!(bitmap.pixels(), vec![0, 0, 255, 255]);
}

#[test]
fn bitmap_import_accepts_unpadded_final_row_and_rejects_invalid_jpeg_quality() {
    use ferroui_base::media::imaging::{BitmapEncoderOptions, JpegBitmapEncoderOptions};
    let bytes = [255, 0, 0, 255, 99, 99, 99, 99, 0, 255, 0, 255];
    let bitmap = Bitmap::from_pixels(
        PixelFormat::RGBA8888,
        AlphaFormat::Premul,
        &bytes,
        PixelSize::new(1, 2),
        Vector::new(96., 96.),
        8,
    );
    assert_eq!(bitmap.pixels(), vec![0, 0, 255, 255, 0, 255, 0, 255]);
    for quality in [-1, 101] {
        let mut output = Vec::new();
        let error = bitmap
            .save(
                &mut output,
                &BitmapEncoderOptions::Jpeg(JpegBitmapEncoderOptions { quality }),
            )
            .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(output.is_empty());
    }
}

#[test]
fn embedded_font_metadata_preserves_real_weight_and_style() {
    use ferroui_base::media::{FontSimulations, FontStyle, FontWeight};
    let manager = fonts::FontManager::default();
    let bold = include_bytes!("../../Skia/FerroUI.Skia/test_assets/assets/Inter-Bold.ttf");
    let italic = include_bytes!("../../Skia/FerroUI.Skia/test_assets/assets/NotoSans-Italic.ttf");
    let face = manager
        .try_create_glyph_typeface_from_stream(&mut &bold[..], FontSimulations::None)
        .unwrap();
    assert_eq!(face.weight(), FontWeight::Bold);
    assert_eq!(face.style(), FontStyle::Normal);
    let face = manager
        .try_create_glyph_typeface_from_stream(&mut &italic[..], FontSimulations::None)
        .unwrap();
    assert_eq!(face.style(), FontStyle::Italic);
}

#[test]
fn opentype_feature_ranges_respect_utf16_owner_offsets() {
    use ferroui_base::media::{text_formatting::TextShaperOptions, FontFeature, FontSimulations};
    use ferroui_base::utilities::ReadOnlyMemory;
    let manager = fonts::FontManager::default();
    let bytes = include_bytes!("../../Skia/FerroUI.Skia/test_assets/assets/NotoSans-Italic.ttf");
    let face = manager
        .try_create_glyph_typeface_from_stream(&mut &bytes[..], FontSimulations::None)
        .unwrap();
    let face = GlyphTypeface::new(face, FontSimulations::None).unwrap();
    let owner = ReadOnlyMemory::from_str("xxfi fixx");
    let text = owner.slice(2, 5);
    let shape = |features| {
        fonts::TextShaper.shape_text(
            &text,
            &TextShaperOptions::with_all(face.clone(), 24., 0, None, 0., 0., features),
        )
    };
    assert_eq!(
        shape(None).length(),
        3,
        "the fixture forms two fi ligatures"
    );
    let disabled = FontFeature {
        tag: "liga".to_owned(),
        value: 0,
        ..Default::default()
    };
    assert_eq!(shape(Some(Rc::new(vec![disabled.clone()]))).length(), 5);
    let ranged = shape(Some(Rc::new(vec![FontFeature {
        start: 2,
        end: 4,
        ..disabled
    }])));
    assert_eq!(ranged.length(), 4, "only the first fi ligature is disabled");
    let clusters: Vec<_> = (0..ranged.length())
        .map(|i| ranged.glyph_infos()[i].glyph_cluster)
        .collect();
    assert_eq!(clusters, [0, 1, 2, 3]);
}

#[test]
fn bitmap_png_compression_options_control_deflate_without_changing_pixels() {
    use ferroui_base::media::imaging::{
        BitmapEncoderOptions, CompressionLevel, PngBitmapEncoderOptions,
    };
    let bitmap = Bitmap::new(PixelSize::new(128, 64), Vector::new(96., 96.));
    let mut pixels = vec![0; 128 * 64 * 4];
    for (i, p) in pixels.chunks_exact_mut(4).enumerate() {
        p.copy_from_slice(&[(i % 16) as u8, ((i / 128) % 8) as u8, 220, 255]);
    }
    bitmap.write(&pixels);
    let mut sizes = Vec::new();
    for (level, deflate_hint) in [
        (CompressionLevel::Optimal, 2),
        (CompressionLevel::Fastest, 0),
        (CompressionLevel::NoCompression, 0),
        (CompressionLevel::SmallestSize, 3),
    ] {
        let mut png = Vec::new();
        bitmap
            .save(
                &mut png,
                &BitmapEncoderOptions::Png(PngBitmapEncoderOptions {
                    compression_level: level,
                }),
            )
            .unwrap();
        let mut offset = 8;
        loop {
            let n = u32::from_be_bytes(png[offset..offset + 4].try_into().unwrap()) as usize;
            if &png[offset + 4..offset + 8] == b"IDAT" {
                assert_eq!(
                    png[offset + 9] >> 6,
                    deflate_hint,
                    "{level:?} must reach zlib"
                );
                break;
            }
            offset += n + 12;
        }
        let decoded = Bitmap::decode(&mut &png[..]).unwrap();
        assert_eq!(decoded.pixels(), pixels);
        sizes.push(png.len());
    }
    assert!(
        sizes[2] > sizes[0] * 10,
        "NoCompression must produce an uncompressed deflate stream"
    );
}

#[test]
fn bitmap_decode_resize_validates_dimensions_and_uses_integer_aspect_sizes() {
    use ferroui_base::media::imaging::{BitmapEncoderOptions, PngBitmapEncoderOptions};
    let bitmap = Bitmap::new(PixelSize::new(7, 5), Vector::new(96., 96.));
    let mut png = Vec::new();
    bitmap
        .save(
            &mut png,
            &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT),
        )
        .unwrap();
    let resized = QuartzRenderInterface
        .load_bitmap_to_width(&mut &png[..], 4, BitmapInterpolationMode::HighQuality)
        .unwrap();
    assert_eq!(resized.pixel_size(), PixelSize::new(4, 2));
    let resized = QuartzRenderInterface
        .load_bitmap_to_height(&mut &png[..], 4, BitmapInterpolationMode::HighQuality)
        .unwrap();
    assert_eq!(resized.pixel_size(), PixelSize::new(5, 4));
    for dimension in [0, -1] {
        let error = QuartzRenderInterface
            .load_bitmap_to_width(
                &mut &png[..],
                dimension,
                BitmapInterpolationMode::LowQuality,
            )
            .err()
            .expect("invalid width rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
        assert!(QuartzRenderInterface
            .load_writeable_bitmap_to_height(
                &mut &png[..],
                dimension,
                BitmapInterpolationMode::LowQuality
            )
            .is_err());
    }
}

#[cfg(feature = "metal")]
#[test]
fn opacity_and_isolated_groups_apply_antialiased_parent_clips_only_once() {
    use ferroui_base::media::imaging::BitmapBlendingMode;
    let mut results = Vec::new();
    for group in 0..3 {
        let bitmap = Bitmap::new(PixelSize::new(32, 32), Vector::new(96., 96.));
        let mut draw = bitmap.drawing(true, None);
        let clip = Geometry::ellipse(Rect::new(2.3, 2.7, 25., 23.));
        draw.push_geometry_clip(&*clip);
        if group == 1 {
            draw.push_opacity(1., None);
        }
        if group == 2 {
            assert!(draw.try_push_compositing_layer(
                BitmapBlendingMode::SourceOver,
                1.,
                Rect::new(0., 0., 32., 32.)
            ));
        }
        draw.draw_rectangle(
            Some(&ImmutableSolidColorBrush::new(Colors::WHITE)),
            None,
            RoundedRect::from_rect(Rect::new(0., 0., 32., 32.)),
            &Default::default(),
        );
        if group == 1 {
            draw.pop_opacity();
        }
        if group == 2 {
            draw.pop_compositing_layer();
        }
        draw.pop_geometry_clip();
        draw.dispose();
        results.push(bitmap.pixels());
    }
    assert!(
        results[0].chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255),
        "fixture exercises fractional clip coverage"
    );
    for result in &results[1..] {
        let error = results[0]
            .iter()
            .zip(result)
            .map(|(a, b)| (*a as i32 - *b as i32).abs())
            .max()
            .unwrap();
        assert!(
            error <= 1,
            "an identity group must preserve parent clip edge coverage; maximum error={error}"
        );
    }
}

#[cfg(feature = "metal")]
#[test]
fn bitmap_blending_options_do_not_change_vector_primitive_blending() {
    use ferroui_base::media::{imaging::BitmapBlendingMode, RenderOptions};
    let target = Bitmap::new(PixelSize::new(4, 4), Vector::new(96., 96.));
    let source = Bitmap::new(PixelSize::new(1, 1), Vector::new(96., 96.));
    source.write(&[255, 0, 0, 255]);
    let mut draw = target.drawing(true, None);
    draw.push_render_options(RenderOptions {
        bitmap_blending_mode: BitmapBlendingMode::Destination,
        ..Default::default()
    });
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::RED)),
        None,
        RoundedRect::from_rect(Rect::new(0., 0., 4., 4.)),
        &Default::default(),
    );
    draw.draw_bitmap(
        &source,
        1.,
        Rect::new(0., 0., 1., 1.),
        Rect::new(0., 0., 4., 4.),
    );
    draw.pop_render_options();
    draw.dispose();
    assert!(
        target
            .pixels()
            .chunks_exact(4)
            .all(|p| p == [0, 0, 255, 255]),
        "Destination applies to the bitmap, while the vector fill still uses SourceOver"
    );
}

#[cfg(feature = "metal")]
#[test]
fn bitmap_quad_edges_preserve_the_same_coverage_as_vector_rectangles() {
    let source = Bitmap::new(PixelSize::new(1, 1), Vector::new(96., 96.));
    source.write(&[255; 4]);
    for transform in [
        Matrix::IDENTITY,
        Matrix::create_rotation(0.17) * Matrix::create_translation(4., 0.),
    ] {
        let mut results = Vec::new();
        for image in [false, true] {
            let target = Bitmap::new(PixelSize::new(40, 40), Vector::new(96., 96.));
            let mut draw = target.drawing(true, None);
            draw.set_transform(transform);
            let rect = Rect::new(4.6, 3.2, 20., 23.4);
            if image {
                draw.draw_bitmap(&source, 1., Rect::new(0., 0., 1., 1.), rect);
            } else {
                draw.draw_rectangle(
                    Some(&ImmutableSolidColorBrush::new(Colors::WHITE)),
                    None,
                    RoundedRect::from_rect(rect),
                    &Default::default(),
                );
            }
            draw.dispose();
            results.push(target.pixels());
        }
        let error = results[0]
            .iter()
            .zip(&results[1])
            .map(|(a, b)| (*a as i32 - *b as i32).abs())
            .max()
            .unwrap();
        assert!(
            error <= 1,
            "bitmap and vector quads must retain fractional edge coverage: {error}"
        );
    }
}

#[cfg(feature = "metal")]
#[test]
fn native_glyph_images_keep_monochrome_and_color_glyphs_upright() {
    use ferroui_base::media::{
        text_formatting::TextShaperOptions, FontSimulations, FontStretch, FontStyle, FontWeight,
        TextHintingMode, TextOptions,
    };
    use ferroui_base::utilities::ReadOnlyMemory;
    let manager = fonts::FontManager::default();
    let face = manager
        .try_create_glyph_typeface(
            "Helvetica",
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
        )
        .unwrap();
    let face = GlyphTypeface::new(face, FontSimulations::None).unwrap();
    let glyph = face.character_to_glyph_map().get_glyph('F' as i32);
    let run = GlyphRun::new(
        &face,
        64.,
        &[GlyphInfo::with_offset(glyph, 0, 50., Vector::ZERO)],
        Point::new(10., 70.),
    );
    for hinting in [TextHintingMode::None, TextHintingMode::Strong] {
        let target = Bitmap::new(PixelSize::new(100, 90), Vector::new(96., 96.));
        let mut draw = target.drawing(true, None);
        draw.push_text_options(TextOptions {
            text_hinting_mode: hinting,
            ..Default::default()
        });
        draw.draw_glyph_run(Some(&ImmutableSolidColorBrush::new(Colors::WHITE)), &run);
        draw.pop_text_options();
        draw.dispose();
        let pixels = target.pixels();
        let rows: Vec<_> = pixels
            .chunks_exact(100 * 4)
            .map(|row| row.chunks_exact(4).filter(|p| p[3] > 128).count())
            .collect();
        let first = rows.iter().position(|n| *n > 0).unwrap();
        let last = rows.iter().rposition(|n| *n > 0).unwrap();
        let third = (last - first + 1) / 3;
        let top: usize = rows[first..first + third].iter().sum();
        let bottom: usize = rows[last + 1 - third..=last].iter().sum();
        // F's long bar is above its stem. Check orientation independently of
        // the native hinting adjustments and antialiasing of either path.
        assert!(
            top * 4 > bottom * 5,
            "upright F with {hinting:?}: {top}/{bottom}"
        );
    }

    let face = manager
        .try_match_character(
            '🚦' as i32,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            None,
            None,
        )
        .unwrap();
    let face = GlyphTypeface::new(face, FontSimulations::None).unwrap();
    let text = ReadOnlyMemory::from_str("🚦");
    let shaped = fonts::TextShaper.shape_text(
        &text,
        &TextShaperOptions::with_all(face.clone(), 64., 0, None, 0., 0., None),
    );
    let run = GlyphRun::new(
        &face,
        64.,
        &shaped.glyph_infos().to_vec(),
        Point::new(10., 70.),
    );
    let target = Bitmap::new(PixelSize::new(100, 90), Vector::new(96., 96.));
    let mut draw = target.drawing(true, None);
    draw.draw_glyph_run(Some(&ImmutableSolidColorBrush::new(Colors::WHITE)), &run);
    draw.dispose();
    let mut red = (0usize, 0usize);
    let mut green = (0usize, 0usize);
    for (index, p) in target.pixels().chunks_exact(4).enumerate() {
        if p[2] > 100 && p[2] as u32 > p[1] as u32 * 2 {
            red.0 += index / 100;
            red.1 += 1;
        }
        if p[1] > 100 && p[1] as u32 > p[2] as u32 * 2 {
            green.0 += index / 100;
            green.1 += 1;
        }
    }
    assert!(red.1 > 10 && green.1 > 10);
    assert!(
        red.0 * green.1 < green.0 * red.1,
        "the red traffic light must remain above green"
    );
}

#[test]
fn embedded_family_matching_keeps_real_faces_and_font_data() {
    use ferroui_base::media::{FontSimulations, FontStretch, FontStyle, FontWeight};
    let manager = fonts::FontManager::default();
    let regular = include_bytes!("../../Skia/FerroUI.Skia/test_assets/assets/Inter-Regular.ttf");
    let bold = include_bytes!("../../Skia/FerroUI.Skia/test_assets/assets/Inter-Bold.ttf");
    let regular_face = manager
        .try_create_glyph_typeface_from_stream(&mut &regular[..], FontSimulations::None)
        .unwrap();
    manager
        .try_create_glyph_typeface_from_stream(&mut &bold[..], FontSimulations::None)
        .unwrap();
    let family = regular_face.family_name();
    for (requested, expected, bytes) in [
        (FontWeight::Normal, FontWeight::Normal, &regular[..]),
        (FontWeight::Bold, FontWeight::Bold, &bold[..]),
        (FontWeight::SemiBold, FontWeight::Bold, &bold[..]),
    ] {
        let face = manager
            .try_create_glyph_typeface(&family, FontStyle::Normal, requested, FontStretch::Normal)
            .unwrap();
        assert_eq!(face.weight(), expected);
        let mut actual = Vec::new();
        face.try_get_stream()
            .expect("family lookup preserves embedded data")
            .read_to_end(&mut actual)
            .unwrap();
        assert_eq!(actual, bytes);
    }
}

#[test]
#[cfg(feature = "metal")]
fn unavailable_metal_and_oversized_surfaces_are_recoverable() {
    let context = QuartzRenderInterface.create_backend_context(None);
    assert!(!context.is_ready_to_create_render_target(&[]));
    let scene = RenderTargetSceneInfo::new(
        PixelSize::new(10, 10),
        1.,
        ferroui_base::rendering::composition::CompositionTransparencyLevel::None,
    );
    assert!(context
        .create_render_target(&[])
        .try_create_drawing_context(&scene)
        .is_err());
    assert!(Bitmap::try_new(PixelSize::new(16385, 1), Vector::new(96., 96.)).is_err());
    let empty = Bitmap::new(PixelSize::new(0, 0), Vector::new(96., 96.));
    assert!(empty.pixels().is_empty());
    let mut draw = empty.drawing(false, None);
    draw.clear(Colors::RED);
    draw.dispose();
    unsafe {
        assert!(ffi::q_font_data(std::ptr::null(), 0).is_null());
        assert!(ffi::q_font_variations(
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            0
        )
        .is_null());
        assert_eq!(ffi::qm_configure(std::ptr::null_mut()), 0);
        assert_eq!(
            ffi::qm_present(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0
            ),
            0
        );
        ffi::q_release(std::ptr::null_mut());
    }
}
