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

#[cfg(feature = "metal")]
#[test]
fn batched_draws_keep_order_across_nested_layers_copies_and_pool_reuse() {
    use ferroui_base::media::imaging::BitmapBlendingMode;
    let target = Bitmap::new(PixelSize::new(64, 64), Vector::new(96., 96.));
    let fill = |draw: &mut drawing::DrawingContext, color, rect| {
        draw.draw_rectangle(
            Some(&ImmutableSolidColorBrush::new(color)),
            None,
            RoundedRect::from_rect(rect),
            &Default::default(),
        );
    };
    let mut draw = target.drawing(true, None);
    draw.clear(Colors::BLUE);
    for _ in 0..12 {
        // Reusing a layer texture must not change an earlier queued image draw.
        draw.push_opacity(0.5, None);
        fill(&mut draw, Colors::RED, Rect::new(4., 4., 12., 12.));
        draw.push_opacity(0.5, None);
        fill(&mut draw, Colors::WHITE, Rect::new(32., 32., 12., 12.));
        draw.pop_opacity();
        draw.pop_opacity();
    }
    let nested = target.pixels();
    let red = &nested[(6 * 64 + 6) * 4..][..4];
    assert!(
        red[2] >= 250 && red[0] <= 2 && red[1] == 0 && red[3] == 255,
        "{red:?}"
    );
    let white = &nested[(34 * 64 + 34) * 4..][..4];
    assert!(
        white[2] >= 245 && white[1] >= 245 && white[0] == 255 && white[3] == 255,
        "{white:?}"
    );
    assert_eq!(&nested[(60 * 64 + 60) * 4..][..4], &[255, 0, 0, 255]);
    // Source replaces even transparent pixels outside the painted region.
    assert!(draw.try_push_compositing_layer(
        BitmapBlendingMode::Source,
        1.,
        Rect::new(0., 0., 64., 64.)
    ));
    fill(&mut draw, Colors::RED, Rect::new(20., 20., 8., 8.));
    draw.pop_compositing_layer();
    draw.dispose();
    let pixels = target.pixels();
    let pixel = |x: usize, y: usize| &pixels[(y * 64 + x) * 4..(y * 64 + x + 1) * 4];
    assert_eq!(pixel(22, 22), &[0, 0, 255, 255]);
    assert_eq!(pixel(6, 6), &[0, 0, 0, 0]);
    assert_eq!(pixel(34, 34), &[0, 0, 0, 0]);
    assert_eq!(pixel(60, 60), &[0, 0, 0, 0]);
}

#[cfg(all(feature = "metal", feature = "media-test-fixtures"))]
#[test]
fn native_glyph_atlas_eviction_preserves_pixels_without_frame_readbacks() {
    const FLAG: &str = "FERRO_QUARTZ_ISOLATED_ATLAS_TEST";
    if std::env::var_os(FLAG).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "parity_tests::native_glyph_atlas_eviction_preserves_pixels_without_frame_readbacks", "--nocapture"])
            .env(FLAG, "1").status().unwrap();
        assert!(status.success());
        return;
    }
    unsafe extern "C" {
        fn qm_fixture_glyph_atlas(enabled: i32, budget: usize);
    }
    unsafe {
        qm_fixture_glyph_atlas(1, 1024 * 1024);
    }
    use ferroui_base::media::{TextHintingMode, TextOptions};
    let manager = fonts::FontManager::default();
    let face = manager
        .try_create_glyph_typeface(
            "Helvetica",
            ferroui_base::media::FontStyle::Normal,
            ferroui_base::media::FontWeight::Normal,
            ferroui_base::media::FontStretch::Normal,
        )
        .unwrap();
    let face = GlyphTypeface::new(face, ferroui_base::media::FontSimulations::None).unwrap();
    let target = Bitmap::new(PixelSize::new(256, 256), Vector::new(96., 96.));
    let paint = |glyph| {
        let run = GlyphRun::new(
            &face,
            192.,
            &[GlyphInfo::with_offset(glyph, 0, 192., Vector::ZERO)],
            Point::new(8., 220.),
        );
        let mut draw = target.drawing(false, None);
        draw.clear(Colors::TRANSPARENT);
        draw.push_text_options(TextOptions {
            text_hinting_mode: TextHintingMode::Light,
            ..Default::default()
        });
        draw.draw_glyph_run(Some(&ImmutableSolidColorBrush::new(Colors::WHITE)), &run);
        draw.pop_text_options();
        draw.dispose();
    };
    let f = face.character_to_glyph_map().get_glyph('F' as i32);
    paint(f);
    let expected = target.pixels();
    let before = crate::metal_statistics();
    for codepoint in 33..800 {
        let glyph = face.character_to_glyph_map().get_glyph(codepoint);
        if glyph != 0 {
            paint(glyph);
        }
    }
    let before_revisit = crate::metal_statistics();
    paint(f);
    let after = crate::metal_statistics();
    assert!(
        after[5] > before_revisit[5],
        "the initial F must have been evicted"
    );
    assert!(
        after[5] > before[5] + 200,
        "exercise multiple atlas pages: {before:?} -> {after:?}"
    );
    assert_eq!(before[4], after[4], "eviction must stay on the GPU");
    assert_eq!(after[6], 0);
    assert_eq!(expected, target.pixels());
}

#[cfg(all(feature = "metal", feature = "media-test-fixtures"))]
#[test]
fn drawable_batches_preserve_layer_growth_and_compute_fallback_without_readbacks() {
    const FLAG: &str = "FERRO_QUARTZ_ISOLATED_DRAWABLE_TEST";
    if std::env::var_os(FLAG).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "parity_tests::drawable_batches_preserve_layer_growth_and_compute_fallback_without_readbacks", "--nocapture"])
            .env(FLAG,"1").status().unwrap();
        assert!(status.success());
        return;
    }
    use ferroui_base::media::{
        immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush},
        GradientSpreadMethod,
    };
    unsafe extern "C" {
        fn qm_fixture_render_target(w: usize, h: usize) -> ffi::Ptr;
    }
    let texture = unsafe { ffi::Owned::new(qm_fixture_render_target(64, 64)) };
    assert!(!texture.0.is_null());
    let surface = unsafe { ffi::qm_surface_external(texture.0) };
    assert!(!surface.is_null());
    let target = Bitmap::from_context(PixelSize::new(64, 64), Vector::new(96., 96.), unsafe {
        ffi::Owned::new(surface)
    });
    let before = crate::metal_statistics();
    let mut draw = target.drawing(false, None);
    draw.clear(Colors::BLUE);
    let fill = |draw: &mut drawing::DrawingContext, color, rect| {
        draw.draw_rectangle(
            Some(&ImmutableSolidColorBrush::new(color)),
            None,
            RoundedRect::from_rect(rect),
            &Default::default(),
        );
    };
    fill(&mut draw, Colors::RED, Rect::new(4., 4., 12., 12.));
    draw.push_opacity(0.5, Some(Rect::new(20., 20., 2., 2.)));
    fill(&mut draw, Colors::RED, Rect::new(20., 20., 12., 12.));
    fill(&mut draw, Colors::WHITE, Rect::new(40., 40., 8., 8.));
    draw.pop_opacity();
    draw.push_opacity(0.5, Some(Rect::new(20., 20., 12., 12.)));
    fill(&mut draw, Colors::WHITE, Rect::new(20., 20., 12., 12.));
    draw.pop_opacity();
    // A drawable without ShaderWrite must preserve prior queued draws when a
    // compute paint moves the frame into private storage.
    let gradient = ImmutableLinearGradientBrush::new(
        &[
            ImmutableGradientStop::new(0., Colors::WHITE),
            ImmutableGradientStop::new(1., Colors::WHITE),
        ],
        1.,
        None,
        None,
        GradientSpreadMethod::Pad,
        None,
        None,
        None,
    );
    draw.draw_rectangle(
        Some(&gradient),
        None,
        RoundedRect::from_rect(Rect::new(50., 4., 10., 10.)),
        &Default::default(),
    );
    draw.dispose();
    let after = crate::metal_statistics();
    assert_eq!(before[4], after[4]);
    assert_eq!(after[6], 0);
    let pixels = target.pixels();
    let check = |x: usize, y: usize, expected: [u8; 4]| {
        let actual = &pixels[(y * 64 + x) * 4..][..4];
        assert!(
            actual
                .iter()
                .zip(expected)
                .all(|(a, b)| (i32::from(*a) - i32::from(b)).abs() <= 1),
            "{x},{y}: {actual:?} vs {expected:?}"
        );
    };
    check(6, 6, [0, 0, 255, 255]);
    check(22, 22, [192, 128, 192, 255]);
    check(42, 42, [255, 128, 128, 255]);
    check(54, 8, [255; 4]);
    check(60, 60, [255, 0, 0, 255]);
}

#[cfg(all(feature = "metal", feature = "media-test-fixtures"))]
#[test]
fn packed_native_masks_match_unatlased_core_text_with_rotation_and_cubic_sampling() {
    const FLAG: &str = "FERRO_QUARTZ_ISOLATED_MASK_REFERENCE";
    if std::env::var_os(FLAG).is_none() {
        let status=std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact","parity_tests::packed_native_masks_match_unatlased_core_text_with_rotation_and_cubic_sampling","--nocapture"])
            .env(FLAG,"1").status().unwrap();
        assert!(status.success());
        return;
    }
    unsafe extern "C" {
        fn qm_fixture_glyph_atlas(enabled: i32, budget: usize);
    }
    use ferroui_base::media::{
        imaging::BitmapInterpolationMode, RenderOptions, TextHintingMode, TextOptions,
    };
    let manager = fonts::FontManager::default();
    let face = manager
        .try_create_glyph_typeface(
            "Helvetica",
            ferroui_base::media::FontStyle::Normal,
            ferroui_base::media::FontWeight::Normal,
            ferroui_base::media::FontStretch::Normal,
        )
        .unwrap();
    let face = GlyphTypeface::new(face, ferroui_base::media::FontSimulations::None).unwrap();
    let glyph = face.character_to_glyph_map().get_glyph('F' as i32);
    let run = GlyphRun::new(
        &face,
        24.,
        &[GlyphInfo::with_offset(glyph, 0, 24., Vector::ZERO)],
        Point::new(20., 48.),
    );
    for transform in [
        Matrix::IDENTITY,
        Matrix::create_scale(2., 2.),
        Matrix::create_rotation(0.23) * Matrix::create_translation(15., 0.),
        Matrix::create_scale(-1., 1.) * Matrix::create_translation(120., 0.),
    ] {
        for interpolation in [
            BitmapInterpolationMode::None,
            BitmapInterpolationMode::LowQuality,
            BitmapInterpolationMode::MediumQuality,
            BitmapInterpolationMode::HighQuality,
        ] {
            let mut outputs = Vec::new();
            for atlas in [0, 1] {
                unsafe {
                    qm_fixture_glyph_atlas(atlas, 16 * 1024 * 1024);
                }
                let bitmap = Bitmap::new(PixelSize::new(160, 160), Vector::new(96., 96.));
                let mut draw = bitmap.drawing(false, None);
                draw.set_transform(transform);
                draw.push_render_options(RenderOptions {
                    bitmap_interpolation_mode: interpolation,
                    ..Default::default()
                });
                draw.push_text_options(TextOptions {
                    text_hinting_mode: TextHintingMode::Light,
                    ..Default::default()
                });
                draw.draw_glyph_run(
                    Some(&ImmutableSolidColorBrush::new(
                        ferroui_base::media::Color::from_argb(200, 210, 70, 130),
                    )),
                    &run,
                );
                draw.pop_text_options();
                draw.pop_render_options();
                draw.dispose();
                outputs.push(bitmap.pixels());
            }
            assert!(outputs[0].chunks_exact(4).any(|p| p[3] > 0));
            let error = outputs[0]
                .iter()
                .zip(&outputs[1])
                .map(|(a, b)| (i32::from(*a) - i32::from(*b)).abs())
                .max()
                .unwrap();
            assert!(error<=1,"atlas changed native mask pixels: {interpolation:?}, {transform:?}, max error={error}");
        }
    }
}

#[cfg(all(feature = "metal", feature = "media-test-fixtures"))]
#[test]
fn fractional_image_quads_match_general_path_coverage() {
    const FLAG: &str = "FERRO_QUARTZ_ISOLATED_IMAGE_QUADS";
    if std::env::var_os(FLAG).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "parity_tests::fractional_image_quads_match_general_path_coverage",
                "--nocapture",
            ])
            .env(FLAG, "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    unsafe extern "C" {
        fn qm_fixture_rect_images(enabled: i32);
    }
    use ferroui_base::media::{imaging::BitmapInterpolationMode, EdgeMode, RenderOptions};
    let source = Bitmap::new(PixelSize::new(8, 8), Vector::new(96., 96.));
    let pixels: Vec<u8> = (0..64).flat_map(|i| [i * 2, i, 64, 160]).collect();
    source.write(&pixels);
    for transform in [
        Matrix::IDENTITY,
        Matrix::create_scale(1.5, 2.) * Matrix::create_translation(0.25, 0.5),
        Matrix::create_scale(-1., 1.) * Matrix::create_translation(70.25, 0.75),
        Matrix::create_scale(1., -1.) * Matrix::create_translation(0.25, 70.75),
        Matrix::create_rotation(0.23) * Matrix::create_translation(15., 0.),
    ] {
        for edge in [EdgeMode::Unspecified, EdgeMode::Aliased] {
            for interpolation in [
                BitmapInterpolationMode::None,
                BitmapInterpolationMode::LowQuality,
                BitmapInterpolationMode::MediumQuality,
                BitmapInterpolationMode::HighQuality,
            ] {
                for clipped in [false, true] {
                    let mut outputs = Vec::new();
                    for fast in [0, 1] {
                        unsafe {
                            qm_fixture_rect_images(fast);
                        }
                        let target = Bitmap::new(PixelSize::new(100, 100), Vector::new(96., 96.));
                        let mut draw = target.drawing(false, None);
                        draw.clear(Colors::WHITE);
                        draw.set_transform(transform);
                        draw.push_render_options(RenderOptions {
                            edge_mode: edge,
                            bitmap_interpolation_mode: interpolation,
                            ..Default::default()
                        });
                        if clipped {
                            draw.push_clip(Rect::new(10.25, 11.75, 24., 22.));
                        }
                        draw.push_opacity(0.7, Some(Rect::new(7., 9., 36., 30.)));
                        draw.draw_bitmap(
                            &source,
                            0.8,
                            Rect::new(1., 1., 6., 5.),
                            Rect::new(7.25, 9.125, 31.5, 25.75),
                        );
                        draw.pop_opacity();
                        // A second atlas view paints beyond its logical canvas.
                        // Its quad must not overwrite the previous group's region.
                        draw.push_opacity(0.6, Some(Rect::new(0., 0., 100., 100.)));
                        draw.draw_bitmap(
                            &source,
                            0.8,
                            Rect::new(1., 1., 6., 5.),
                            Rect::new(-15.25, -10.5, 150.5, 120.25),
                        );
                        draw.pop_opacity();
                        if clipped {
                            draw.pop_clip();
                        }
                        draw.pop_render_options();
                        draw.dispose();
                        outputs.push(target.pixels());
                    }
                    assert!(outputs[0].chunks_exact(4).any(|p| p[0] < 240));
                    let error = outputs[0]
                        .iter()
                        .zip(&outputs[1])
                        .map(|(a, b)| (i32::from(*a) - i32::from(*b)).abs())
                        .max()
                        .unwrap();
                    assert!(error <= 1, "image quad changed pixels: {transform:?}, {edge:?}, {interpolation:?}, clipped={clipped}, error={error}");
                }
            }
        }
    }
    unsafe {
        qm_fixture_rect_images(1);
    }
}

#[cfg(feature = "metal")]
#[test]
fn plain_rectangles_match_native_geometry_at_fractional_edges() {
    use ferroui_base::media::{EdgeMode, RenderOptions};
    for rect in [
        Rect::new(7.25, 9.125, 31.5, 25.75),
        Rect::new(7., 9., 32., 26.),
    ] {
        let geometry = Geometry::rectangle(rect);
        let brush =
            ImmutableSolidColorBrush::new(ferroui_base::media::Color::from_argb(160, 90, 130, 200));
        for transform in [
            Matrix::IDENTITY,
            Matrix::create_scale(1.5, 2.),
            Matrix::create_scale(-1., 1.) * Matrix::create_translation(70.25, 0.75),
            Matrix::create_rotation(0.23) * Matrix::create_translation(15., 0.),
        ] {
            for edge in [EdgeMode::Unspecified, EdgeMode::Aliased] {
                let mut outputs = Vec::new();
                for rectangle in [false, true] {
                    let target = Bitmap::new(PixelSize::new(100, 100), Vector::new(96., 96.));
                    let mut draw = target.drawing(false, None);
                    draw.clear(Colors::WHITE);
                    draw.set_transform(transform);
                    draw.push_render_options(RenderOptions {
                        edge_mode: edge,
                        ..Default::default()
                    });
                    if rectangle {
                        draw.draw_rectangle(
                            Some(&brush),
                            None,
                            RoundedRect::from_rect(rect),
                            &ferroui_base::media::BoxShadows::default(),
                        );
                    } else {
                        draw.draw_geometry(Some(&brush), None, &*geometry);
                    }
                    draw.pop_render_options();
                    draw.dispose();
                    outputs.push(target.pixels());
                }
                assert_eq!(
                    outputs[0], outputs[1],
                    "rectangle changed coverage: {transform:?}, {edge:?}"
                );
            }
        }
    }
}

#[test]
fn immutable_shaping_attributes_preserve_complex_script_glyphs_and_context() {
    use ferroui_base::media::{text_formatting::TextShaperOptions, FontFeature, FontSimulations};
    use ferroui_base::utilities::ReadOnlyMemory;
    let manager = fonts::FontManager::default();
    for (bytes, text, bidi) in [
        (
            &include_bytes!("../../Skia/FerroUI.Skia/test_assets/assets/Inter-Regular.ttf")[..],
            "xxoffice affiné 012xx",
            0,
        ),
        (
            &include_bytes!(
                "../../Skia/FerroUI.Skia/test_assets/assets/NotoSansArabic-Regular.ttf"
            )[..],
            "xxمَرْحَبًا بالعالم 012xx",
            1,
        ),
        (
            &include_bytes!(
                "../../Skia/FerroUI.Skia/test_assets/assets/NotoSansHebrew-Regular.ttf"
            )[..],
            "xxשָׁלוֹם עולם 012xx",
            1,
        ),
    ] {
        let platform = manager
            .try_create_glyph_typeface_from_stream(&mut &bytes[..], FontSimulations::None)
            .unwrap();
        let face = GlyphTypeface::new(platform, FontSimulations::None).unwrap();
        let owner = ReadOnlyMemory::from_str(text);
        // An empty feature range selects the mutable implementation without
        // changing any attributes. It is the previous Core Text reference.
        let unchanged = Some(Rc::new(vec![FontFeature {
            tag: "liga".into(),
            value: 0,
            start: owner.len() as i32,
            end: owner.len() as i32,
        }]));
        for input in [owner.clone(), owner.slice(2, owner.len() - 4)] {
            let optimized = fonts::TextShaper.shape_text(
                &input,
                &TextShaperOptions::with_all(face.clone(), 24., bidi, None, 0., 0., None),
            );
            let reference = fonts::TextShaper.shape_text(
                &input,
                &TextShaperOptions::with_all(
                    face.clone(),
                    24.,
                    bidi,
                    None,
                    0.,
                    0.,
                    unchanged.clone(),
                ),
            );
            assert!(optimized.length() > 0);
            assert_eq!(optimized.length(), reference.length());
            for (a, b) in optimized
                .glyph_infos()
                .iter()
                .zip(reference.glyph_infos().iter())
            {
                assert_eq!(
                    (
                        a.glyph_index,
                        a.glyph_cluster,
                        a.glyph_advance,
                        a.glyph_offset
                    ),
                    (
                        b.glyph_index,
                        b.glyph_cluster,
                        b.glyph_advance,
                        b.glyph_offset
                    ),
                    "changed native shaping: {text}"
                );
            }
        }
    }
}

#[cfg(all(feature = "metal", feature = "media-test-fixtures"))]
#[test]
fn fixed_cache_keys_preserve_geometry_when_all_hashes_collide() {
    const FLAG: &str = "FERRO_QUARTZ_ISOLATED_FIXED_KEYS";
    if std::env::var_os(FLAG).is_none() {
        assert!(std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "parity_tests::fixed_cache_keys_preserve_geometry_when_all_hashes_collide",
                "--nocapture"
            ])
            .env(FLAG, "1")
            .status()
            .unwrap()
            .success());
        return;
    }
    unsafe extern "C" {
        fn qm_fixture_key_collisions(enabled: i32);
    }
    use ferroui_base::media::{EdgeMode, FillRule, RenderOptions};
    let mut path = kurbo::BezPath::new();
    path.move_to((0., 0.));
    path.curve_to((2., -4.), (24., 26.), (28., 12.));
    path.line_to((10., 26.));
    path.close_path();
    let geometries = [
        Geometry::new(path.clone(), FillRule::NonZero),
        Geometry::new(path, FillRule::EvenOdd),
    ];
    let brush =
        ImmutableSolidColorBrush::new(ferroui_base::media::Color::from_argb(170, 90, 130, 200));
    let mut outputs = Vec::new();
    for collide in [0, 1, 0] {
        unsafe {
            qm_fixture_key_collisions(collide);
        }
        let mut frames = Vec::new();
        for width in [96, 128] {
            let target = Bitmap::new(PixelSize::new(width, 96), Vector::new(96., 96.));
            let mut draw = target.drawing(false, None);
            draw.clear(Colors::WHITE);
            for pass in 0..2 {
                for i in 0..24 {
                    draw.set_transform(Matrix::create_translation(
                        (i % 6) as f64 * 17. + 0.25,
                        (i / 6) as f64 * 17. + pass as f64 * 0.5,
                    ));
                    draw.push_render_options(RenderOptions {
                        edge_mode: if i % 2 == 0 {
                            EdgeMode::Aliased
                        } else {
                            EdgeMode::Unspecified
                        },
                        ..Default::default()
                    });
                    draw.draw_geometry(Some(&brush), None, &*geometries[(i % 2) as usize]);
                    draw.draw_rectangle(
                        Some(&brush),
                        None,
                        RoundedRect::from_radius_xy(
                            Rect::new(1., 2., 20., 14.),
                            2. + (i % 3) as f64,
                            3. + (i % 4) as f64,
                        ),
                        &Default::default(),
                    );
                    draw.pop_render_options();
                }
            }
            draw.dispose();
            frames.push(target.pixels());
        }
        outputs.push(frames);
    }
    assert_eq!(
        outputs[0], outputs[1],
        "hash collisions must compare full owned keys"
    );
    assert_eq!(
        outputs[0], outputs[2],
        "clearing must release and rebuild owned keys safely"
    );
}

#[cfg(all(feature = "metal", feature = "media-test-fixtures"))]
#[test]
fn unhinted_empty_groups_allocate_no_textures_and_preserve_source_semantics() {
    const FLAG: &str = "FERRO_QUARTZ_ISOLATED_LAZY_GROUPS";
    if std::env::var_os(FLAG).is_none() {
        assert!(std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "parity_tests::unhinted_empty_groups_allocate_no_textures_and_preserve_source_semantics", "--nocapture"])
            .env(FLAG, "1").status().unwrap().success());
        return;
    }
    unsafe extern "C" {
        fn qm_statistics(out: *mut u64, count: usize);
    }
    let statistics = || {
        let mut out = [0; 9];
        unsafe {
            qm_statistics(out.as_mut_ptr(), out.len());
        }
        out
    };
    let target = Bitmap::new(PixelSize::new(64, 64), Vector::new(96., 96.));
    let mut draw = target.drawing(false, None);
    draw.clear(Colors::WHITE);
    let before = statistics();
    for _ in 0..32 {
        draw.push_opacity(0.5, None);
        draw.push_opacity(0.5, None);
        draw.pop_opacity();
        draw.pop_opacity();
    }
    assert_eq!(
        before[8],
        statistics()[8],
        "empty groups must stay metadata-only"
    );
    draw.push_opacity(0.5, None);
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::BLUE)),
        None,
        RoundedRect::from_rect(Rect::new(7.25, 9.125, 13.5, 17.75)),
        &Default::default(),
    );
    draw.pop_opacity();
    draw.dispose();
    let after = statistics();
    assert_eq!(before[4], after[4]);
    assert_eq!(after[6], 0);
    let pixels = target.pixels();
    let actual = &pixels[(12 * 64 + 10) * 4..][..4];
    assert!(
        actual
            .iter()
            .zip([255u8, 128, 128, 255])
            .all(|(a, b)| (i32::from(*a) - i32::from(b)).abs() <= 1),
        "{actual:?}"
    );
    let mut draw = target.drawing(false, None);
    draw.clear(Colors::WHITE);
    target.context(|c| unsafe {
        ffi::q_begin_layer(c, 1., 17);
        ffi::q_end_layer(c);
    });
    draw.dispose();
    assert!(
        target.pixels().iter().all(|x| *x == 0),
        "an empty Source group still replaces the full canvas"
    );
}

#[cfg(all(feature = "metal", feature = "media-test-fixtures"))]
#[test]
fn merged_drawable_groups_preserve_order_across_atlas_pages_and_dependencies() {
    const FLAG: &str = "FERRO_QUARTZ_ISOLATED_MERGED_GROUPS";
    if std::env::var_os(FLAG).is_none() {
        assert!(std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "parity_tests::merged_drawable_groups_preserve_order_across_atlas_pages_and_dependencies", "--nocapture"])
            .env(FLAG, "1").status().unwrap().success());
        return;
    }
    unsafe extern "C" {
        fn qm_fixture_render_target(w: usize, h: usize) -> ffi::Ptr;
        fn qm_fixture_deferred_merge(enabled: i32);
        fn qm_fixture_encoded_batches() -> u64;
    }
    let mut outputs = Vec::new();
    let mut batches = Vec::new();
    for merge in [0, 1] {
        unsafe {
            qm_fixture_deferred_merge(merge);
        }
        let texture = unsafe { ffi::Owned::new(qm_fixture_render_target(128, 128)) };
        assert!(!texture.0.is_null());
        let surface = unsafe { ffi::qm_surface_external(texture.0) };
        assert!(!surface.is_null());
        let target =
            Bitmap::from_context(PixelSize::new(128, 128), Vector::new(96., 96.), unsafe {
                ffi::Owned::new(surface)
            });
        let before_stats = crate::metal_statistics();
        let before = unsafe { qm_fixture_encoded_batches() };
        let mut draw = target.drawing(false, None);
        draw.clear(Colors::WHITE);
        for i in 0..80 {
            if i == 31 {
                draw.draw_rectangle(
                    Some(&ImmutableSolidColorBrush::new(Colors::RED)),
                    None,
                    RoundedRect::from_rect(Rect::new(2., 2., 100., 90.)),
                    &Default::default(),
                );
            }
            if i == 40 {
                draw.push_clip(Rect::new(8.25, 7.75, 100., 100.));
            }
            draw.push_opacity(
                0.1 + (i % 3) as f64 * 0.1,
                Some(Rect::new(0., 0., 128., 128.)),
            );
            let brush = ImmutableSolidColorBrush::new(ferroui_base::media::Color::from_argb(
                190,
                (i * 17 % 256) as u8,
                (i * 39 % 256) as u8,
                (i * 73 % 256) as u8,
            ));
            draw.draw_rectangle(
                Some(&brush),
                None,
                RoundedRect::from_radius_xy(
                    Rect::new(10.25 + (i % 9) as f64, 12.5 + (i % 7) as f64, 72., 56.),
                    5.,
                    7.,
                ),
                &Default::default(),
            );
            if i == 63 {
                draw.push_opacity(0.4, None);
                draw.draw_rectangle(
                    Some(&ImmutableSolidColorBrush::new(Colors::BLUE)),
                    None,
                    RoundedRect::from_rect(Rect::new(30., 30., 50., 40.)),
                    &Default::default(),
                );
                draw.pop_opacity();
            }
            draw.pop_opacity();
            if i == 40 {
                draw.pop_clip();
            }
        }
        draw.dispose();
        let after = unsafe { qm_fixture_encoded_batches() };
        let after_stats = crate::metal_statistics();
        assert_eq!(before_stats[4], after_stats[4]);
        assert_eq!(after_stats[6], 0);
        batches.push(after - before);
        outputs.push(target.pixels());
    }
    assert_eq!(
        outputs[0], outputs[1],
        "merging may not change ordered source-over composition"
    );
    assert!(
        batches[1] < batches[0],
        "merging must reduce encoded batches: {batches:?}"
    );
}

#[cfg(all(feature = "metal", feature = "media-test-fixtures"))]
#[test]
fn compact_atlas_clears_do_not_leak_reused_pixels_or_late_regions() {
    const FLAG: &str = "FERRO_QUARTZ_ISOLATED_PAGE_CLEAR";
    if std::env::var_os(FLAG).is_none() {
        assert!(std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "parity_tests::compact_atlas_clears_do_not_leak_reused_pixels_or_late_regions",
                "--nocapture"
            ])
            .env(FLAG, "1")
            .status()
            .unwrap()
            .success());
        return;
    }
    unsafe extern "C" {
        fn qm_fixture_render_target(w: usize, h: usize) -> ffi::Ptr;
        fn qm_fixture_layer_atlas(enabled: i32);
        fn qm_fixture_atlas_region_clears() -> u64;
    }
    let fill = |draw: &mut drawing::DrawingContext, color, rect| {
        draw.draw_rectangle(
            Some(&ImmutableSolidColorBrush::new(color)),
            None,
            RoundedRect::from_rect(rect),
            &Default::default(),
        );
    };
    let mut outputs = Vec::new();
    for atlas in [0, 1] {
        unsafe {
            qm_fixture_layer_atlas(atlas);
        }
        let texture = unsafe { ffi::Owned::new(qm_fixture_render_target(128, 128)) };
        assert!(!texture.0.is_null());
        let raw = unsafe { ffi::qm_surface_external(texture.0) };
        assert!(!raw.is_null());
        let target =
            Bitmap::from_context(PixelSize::new(128, 128), Vector::new(96., 96.), unsafe {
                ffi::Owned::new(raw)
            });
        // Poison both pages, finish their readers, and let the pool reuse them.
        let mut draw = target.drawing(false, None);
        draw.clear(Colors::WHITE);
        for _ in 0..80 {
            draw.push_opacity(1., Some(Rect::new(0., 0., 128., 128.)));
            fill(&mut draw, Colors::BLUE, Rect::new(0., 0., 128., 128.));
            draw.pop_opacity();
        }
        draw.dispose();
        let before_stats = crate::metal_statistics();
        let before_clears = unsafe { qm_fixture_atlas_region_clears() };
        let mut draw = target.drawing(false, None);
        draw.clear(Colors::WHITE);
        // The first view is empty; its region must still be cleared before
        // composition. Later allocations must preserve earlier regions.
        draw.push_opacity(0.5, Some(Rect::new(0., 0., 128., 128.)));
        draw.pop_opacity();
        for i in 0..80 {
            draw.push_opacity(
                0.3 + (i % 3) as f64 * 0.1,
                Some(Rect::new(0., 0., 128., 128.)),
            );
            fill(&mut draw, Colors::RED, Rect::new(8., 8., 10., 10.));
            fill(&mut draw, Colors::BLUE, Rect::new(72.25, 74.25, 12.5, 10.5));
            if i % 9 == 0 {
                draw.draw_rectangle(
                    Some(&ImmutableSolidColorBrush::new(Colors::GREEN)),
                    None,
                    RoundedRect::from_radius_xy(Rect::new(32.25, 32.5, 14., 14.), 3., 4.),
                    &Default::default(),
                );
            }
            if i == 63 {
                draw.push_opacity(0.5, None);
                fill(&mut draw, Colors::GREEN, Rect::new(25., 70., 8., 8.));
                draw.pop_opacity();
            }
            draw.pop_opacity();
            if i == 37 {
                // A GPU snapshot drains the drawable and closes the atlas
                // pass; later allocations must preserve the existing page.
                let snapshot =
                    target.context(|c| unsafe { ffi::Owned::new(ffi::qm_layer_snapshot(c)) });
                assert!(!snapshot.0.is_null());
                drop(snapshot);
            }
        }
        draw.dispose();
        let clears = unsafe { qm_fixture_atlas_region_clears() } - before_clears;
        assert_eq!(
            clears,
            if atlas == 0 { 0 } else { 81 },
            "clear each region once, including the initial empty view"
        );
        let after_stats = crate::metal_statistics();
        assert_eq!(before_stats[4], after_stats[4]);
        assert_eq!(after_stats[6], 0);
        let pixels = target.pixels();
        assert_eq!(
            &pixels[(55 * 128 + 55) * 4..][..4],
            &[255; 4],
            "transparent gaps must not reveal the poisoned page"
        );
        outputs.push(pixels);
    }
    assert_eq!(
        outputs[0], outputs[1],
        "atlas views must match independent layer textures exactly"
    );
    unsafe {
        qm_fixture_layer_atlas(1);
    }
}

#[cfg(all(feature = "metal", feature = "media-test-fixtures"))]
#[test]
fn compact_draw_counts_cross_recording_and_deferred_merge_limits() {
    const FLAG: &str = "FERRO_QUARTZ_ISOLATED_COMPACT_COUNTS";
    if std::env::var_os(FLAG).is_none() {
        assert!(std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "parity_tests::compact_draw_counts_cross_recording_and_deferred_merge_limits",
                "--nocapture"
            ])
            .env(FLAG, "1")
            .status()
            .unwrap()
            .success());
        return;
    }
    unsafe extern "C" {
        fn qm_fixture_render_target(w: usize, h: usize) -> ffi::Ptr;
    }
    let texture = unsafe { ffi::Owned::new(qm_fixture_render_target(64, 64)) };
    assert!(!texture.0.is_null());
    let surface = unsafe { ffi::qm_surface_external(texture.0) };
    assert!(!surface.is_null());
    let target = Bitmap::from_context(PixelSize::new(64, 64), Vector::new(96., 96.), unsafe {
        ffi::Owned::new(surface)
    });
    let before = crate::metal_statistics();
    let mut draw = target.drawing(false, None);
    draw.clear(Colors::WHITE);
    let mut expected = vec![255; 64 * 64 * 4];
    // Cross the 4096-command limit twice, retaining source-over order and
    // exact counts when full QParams are absent from compact recording.
    for i in 0..8200usize {
        let pixel = i % (64 * 64);
        let (red, green, blue) = ((i * 17) as u8, (i * 39) as u8, (i * 73) as u8);
        let brush =
            ImmutableSolidColorBrush::new(ferroui_base::media::Color::from_rgb(red, green, blue));
        draw.draw_rectangle(
            Some(&brush),
            None,
            RoundedRect::from_rect(Rect::new((pixel % 64) as f64, (pixel / 64) as f64, 1., 1.)),
            &Default::default(),
        );
        expected[pixel * 4..pixel * 4 + 4].copy_from_slice(&[blue, green, red, 255]);
    }
    draw.dispose();
    let after = crate::metal_statistics();
    assert_eq!(before[4], after[4], "drawing may not read back pixels");
    assert_eq!(after[6], 0);
    assert_eq!(
        target.pixels(),
        expected,
        "every compact draw must survive batch boundaries"
    );
}
