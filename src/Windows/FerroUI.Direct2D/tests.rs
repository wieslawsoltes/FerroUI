use super::*;
use ferroui_base::{
    media::{
        immutable::ImmutableSolidColorBrush, text_formatting::TextShaperOptions, Colors,
        FontSimulations, FontStretch, FontStyle, FontWeight,
    },
    utilities::ReadOnlyMemory,
};
use ferroui_base::{Matrix, RoundedRect};
#[test]
fn positioned_cache_keys_all_inputs_and_keeps_evicted_runs_alive() {
    unsafe {
        let name = std::ffi::CString::new("Segoe UI").unwrap();
        let font = ffi::Owned::new(ffi::d2_font(name.as_ptr(), 400, 0, 5));
        let ids = [36u16, 37];
        let positions = [2., 20., 10., 20.];
        let advances = [8., 8.];
        let make = |size, ids: &[u16; 2], positions: &[f64; 4], advances: *const f64| {
            ffi::Owned::new(ffi::d2_positioned_glyph_run(
                font.0,
                size,
                ids.as_ptr(),
                positions.as_ptr(),
                2,
                advances,
            ))
        };
        // First use stays on probation; repeated text is admitted to the LRU.
        let probation = make(16., &ids, &positions, advances.as_ptr());
        let original = make(16., &ids, &positions, advances.as_ptr());
        assert_ne!(
            probation.0, original.0,
            "First-use runs must stay out of the LRU"
        );
        drop(probation);
        let hit = make(16., &ids, &positions, advances.as_ptr());
        assert_eq!(
            original.0, hit.0,
            "Identical positioned runs should share native data"
        );
        for changed in [
            make(17., &ids, &positions, advances.as_ptr()),
            make(16., &[36, 38], &positions, advances.as_ptr()),
            make(16., &ids, &[3., 20., 10., 20.], advances.as_ptr()),
            make(16., &ids, &positions, [7., 8.].as_ptr()),
            make(16., &ids, &positions, std::ptr::null()),
        ] {
            assert_ne!(
                original.0, changed.0,
                "A distinct input must not alias the cached run"
            );
        }
        let render = |run: &ffi::Owned| {
            let bitmap = Bitmap::new(PixelSize::new(64, 32), Vector::new(96., 96.));
            bitmap.context(|c| ffi::d2_draw_positioned_glyph_run(c, run.0, 0., 0, 0., 0., 0., 1.));
            bitmap.pixels()
        };
        let expected = render(&original);
        assert!(expected.iter().any(|&byte| byte != 0));
        for i in 0..1200 {
            // Repeated use admits these entries, filling the bounded LRU.
            for _ in 0..2 {
                drop(make(
                    16.,
                    &ids,
                    &[2. + i as f64, 20., 10. + i as f64, 20.],
                    advances.as_ptr(),
                ));
            }
        }
        assert_eq!(
            expected,
            render(&original),
            "Eviction must retain externally owned runs"
        );
        let recreated = make(16., &ids, &positions, advances.as_ptr());
        assert_ne!(
            original.0, recreated.0,
            "The oldest run should have been evicted"
        );
        drop(hit);
        drop(font);
        assert_eq!(expected, render(&recreated), "Runs outlive the font cache");
        assert_eq!(expected, render(&original));
    }
}

#[test]
fn combined_glyph_draws_preserve_pixels_clipping_and_order() {
    let face = face();
    for scale in [1., 1.25] {
        for alias in [false, true] {
            for overlap in [false, true] {
                let render = |flush: bool| {
                    let bitmap = Bitmap::new(
                        PixelSize::new(360, 180),
                        Vector::new(96. * scale, 96. * scale),
                    );
                    let mut draw = bitmap.drawing(false, None);
                    draw.clear(Colors::WHITE);
                    draw.push_text_options(ferroui_base::media::TextOptions {
                        text_rendering_mode: if alias {
                            ferroui_base::media::TextRenderingMode::Alias
                        } else {
                            ferroui_base::media::TextRenderingMode::Antialias
                        },
                        baseline_pixel_alignment:
                            ferroui_base::media::BaselinePixelAlignment::Unaligned,
                        ..Default::default()
                    });
                    draw.push_clip(Rect::new(0.25, 0.5, 285., 135.));
                    for i in 0..18 {
                        let word = format!("Item {i}-4");
                        let shaped = fonts::TextShaper.shape_text(
                            &ReadOnlyMemory::from_str(&word),
                            &TextShaperOptions::with_all(face.clone(), 12., 0, None, 0., 0., None),
                        );
                        let glyphs: Vec<_> = (0..shaped.length())
                            .map(|g| shaped.glyph_infos()[g])
                            .collect();
                        let run = GlyphRun::new(&face, 12., &glyphs, Point::new(1.125, 15.25));
                        let x = if overlap {
                            (i % 3) * 14
                        } else {
                            [0, 190, 95][i % 3]
                        };
                        draw.set_transform(Matrix::create_translation(
                            x as f64,
                            ((i / 3) * 22) as f64,
                        ));
                        draw.push_clip(Rect::new(0., 0., if i % 4 == 0 { 24. } else { 95. }, 22.));
                        let color = if i % 7 == 0 {
                            Colors::RED
                        } else {
                            Colors::BLACK
                        };
                        if flush {
                            bitmap.context(|c| unsafe {
                                let native = ffi::Owned::new(ffi::d2_positioned_glyph_run(
                                    run.font.raw(),
                                    run.size,
                                    run.ids.as_ptr(),
                                    run.positions.as_ptr(),
                                    run.ids.len(),
                                    std::ptr::null(),
                                ));
                                ffi::d2_draw_positioned_glyph_run(
                                    c,
                                    native.0,
                                    0.,
                                    i32::from(alias),
                                    if color == Colors::RED { 1. } else { 0. },
                                    0.,
                                    0.,
                                    0.7,
                                );
                            });
                        } else {
                            draw.draw_glyph_run(
                                Some(&ImmutableSolidColorBrush::with_opacity(color, 0.7)),
                                &run,
                            );
                        }
                        draw.pop_clip();
                    }
                    draw.pop_clip();
                    draw.pop_text_options();
                    draw.dispose();
                    bitmap.pixels()
                };
                assert_eq!(render(false),render(true),"Glyph combining changed pixels: scale={scale}, alias={alias}, overlap={overlap}");
            }
        }
    }
}

#[test]
fn certified_simple_shaping_matches_full_directwrite_exactly() {
    extern "C" fn emit(p: ffi::Ptr, g: ffi::Glyph) {
        unsafe {
            (*(p as *mut Vec<ffi::Glyph>)).push(g);
        }
    }
    let manager = fonts::FontManager::default();
    let mut corpus = vec![
        "Item 123-4".to_owned(),
        "AV office ffi WA".to_owned(),
        "1234567890".to_owned(),
        "Arabic العربية".to_owned(),
        "a\u{301}😀".to_owned(),
        "".to_owned(),
    ];
    let mut random = 17u32;
    for _ in 0..120 {
        let mut word = String::new();
        for _ in 0..32 {
            random = random.wrapping_mul(1664525).wrapping_add(1013904223);
            word.push((32 + random % 95) as u8 as char);
        }
        corpus.push(word);
    }
    for family in ["Segoe UI", "Arial", "Segoe UI Emoji"] {
        let face = manager
            .try_create_glyph_typeface(
                family,
                FontStyle::Normal,
                FontWeight::Normal,
                FontStretch::Normal,
            )
            .unwrap();
        let font = face
            .as_any()
            .downcast_ref::<fonts::Typeface>()
            .unwrap()
            .font
            .raw();
        for word in &corpus {
            let text: Vec<_> = word.encode_utf16().collect();
            for size in [7.25, 12., 17.3, 64.125] {
                for (start, length) in [
                    (0, text.len()),
                    (text.len().min(2), text.len().saturating_sub(4)),
                ] {
                    for (language, rtl, features) in [
                        ("en-GB", 0, 0),
                        ("en-US", 0, 0),
                        ("tr-TR", 0, 0),
                        ("ar-SA", 1, 0),
                        ("en-US", 0, 1),
                    ] {
                        let language = std::ffi::CString::new(language).unwrap();
                        let run = |reference: bool| {
                            let mut glyphs = Vec::<ffi::Glyph>::new();
                            let shape = if reference {
                                ffi::d2_shape_reference
                            } else {
                                ffi::d2_shape
                            };
                            let result = unsafe {
                                shape(
                                    font,
                                    text.as_ptr(),
                                    text.len(),
                                    start,
                                    length,
                                    language.as_ptr(),
                                    size,
                                    rtl,
                                    [u32::from_be_bytes(*b"kern")].as_ptr(),
                                    [0].as_ptr(),
                                    [0].as_ptr(),
                                    [text.len() as u32].as_ptr(),
                                    features,
                                    &mut glyphs as *mut _ as ffi::Ptr,
                                    emit,
                                )
                            };
                            assert_ne!(result, 0);
                            glyphs
                        };
                        assert_eq!(run(false),run(true),"{family}: {word:?}, size={size}, slice={start}/{length}, language={language:?}, rtl={rtl}, features={features}");
                    }
                }
            }
        }
    }
}

#[test]
fn natural_advances_keep_the_original_explicit_glyph_positions() {
    let manager = fonts::FontManager::default();
    for family in ["Segoe UI", "Arial", "Segoe UI Emoji"] {
        let face = GlyphTypeface::new(
            manager
                .try_create_glyph_typeface(
                    family,
                    FontStyle::Normal,
                    FontWeight::Normal,
                    FontStretch::Normal,
                )
                .unwrap(),
            FontSimulations::None,
        )
        .unwrap();
        for word in ["office AV 123.45", "😀😀", "WAVY fi ffi 987"] {
            let shaped = fonts::TextShaper.shape_text(
                &ReadOnlyMemory::from_str(word),
                &TextShaperOptions::with_all(face.clone(), 12., 0, None, 0., 0., None),
            );
            let glyphs: Vec<_> = (0..shaped.length())
                .map(|i| shaped.glyph_infos()[i])
                .collect();
            let run = GlyphRun::new(&face, 12., &glyphs, Point::new(4.125, 20.));
            let actual = Bitmap::new(PixelSize::new(160, 48), Vector::new(96., 96.));
            let expected = Bitmap::new(PixelSize::new(160, 48), Vector::new(96., 96.));
            let mut draw = actual.drawing(false, None);
            draw.clear(Colors::TRANSPARENT);
            draw.draw_glyph_run(Some(&ImmutableSolidColorBrush::new(Colors::BLACK)), &run);
            draw.dispose();
            expected.context(|c| unsafe {
                ffi::d2_glyphs(
                    c,
                    run.font.raw(),
                    run.size,
                    run.ids.as_ptr(),
                    run.positions.as_ptr(),
                    run.ids.len(),
                    0.,
                    0.,
                    0.,
                    1.,
                );
            });
            assert_eq!(
                actual.pixels(),
                expected.pixels(),
                "Natural advances moved glyphs in {family}: {word}"
            );
        }
    }
}

#[test]
fn lazy_text_clips_match_materialized_clips_at_fractional_dpi_and_transforms() {
    let face = face();
    let shaped = fonts::TextShaper.shape_text(
        &ReadOnlyMemory::from_str("office"),
        &TextShaperOptions::with_all(face.clone(), 16., 0, None, 0., 0., None),
    );
    let glyphs: Vec<_> = (0..shaped.length())
        .map(|i| shaped.glyph_infos()[i])
        .collect();
    let run = GlyphRun::new(&face, 16., &glyphs, Point::new(2., 20.));
    for transform in [
        Matrix::IDENTITY,
        Matrix::create_translation(5.25, 2.5),
        Matrix::new(1.25, 0., 0., 1.25, 4.5, 3.25),
        Matrix::new(-1., 0., 0., 1., 60., 0.),
        Matrix::new(0.9, 0.2, -0.3, 1.1, 20., 2.),
    ] {
        for clip in [
            Rect::new(6.25, 2.5, 20.5, 12.5),
            Rect::new(-40., -40., 120., 120.),
            Rect::new(0., 0., 64., 32.),
        ] {
            for alias in [false, true] {
                let render = |force: bool| {
                    let bitmap = Bitmap::new(PixelSize::new(96, 64), Vector::new(96., 96.));
                    let mut draw = bitmap.drawing(false, None);
                    draw.clear(Colors::TRANSPARENT);
                    draw.set_transform(transform);
                    if alias {
                        draw.push_text_options(ferroui_base::media::TextOptions {
                            text_rendering_mode: ferroui_base::media::TextRenderingMode::Alias,
                            ..Default::default()
                        });
                    }
                    draw.push_clip(clip);
                    if force {
                        bitmap.context(|_| {});
                    }
                    draw.draw_glyph_run(Some(&ImmutableSolidColorBrush::new(Colors::BLACK)), &run);
                    draw.pop_clip();
                    if alias {
                        draw.pop_text_options();
                    }
                    draw.dispose();
                    bitmap.pixels()
                };
                assert_eq!(
                    render(false),
                    render(true),
                    "Lazy clip changed pixels at {transform:?}, {clip:?}, alias={alias}"
                );
            }
        }
    }
}

#[test]
fn serialized_native_bridge_supports_concurrent_targets_and_shared_glyphs() {
    let face = face();
    let shaped = fonts::TextShaper.shape_text(
        &ReadOnlyMemory::from_str("Native"),
        &TextShaperOptions::with_all(face.clone(), 14., 0, None, 0., 0., None),
    );
    let glyphs: Vec<_> = (0..shaped.length())
        .map(|i| shaped.glyph_infos()[i])
        .collect();
    let run = Arc::new(GlyphRun::new(&face, 14., &glyphs, Point::new(2., 18.)));
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let run = run.clone();
            std::thread::spawn(move || {
                let bitmap = Bitmap::new(PixelSize::new(64, 32), Vector::new(96., 96.));
                for _ in 0..20 {
                    let mut draw = bitmap.drawing(false, None);
                    draw.clear(Colors::WHITE);
                    draw.push_clip(Rect::new(0., 0., 64., 32.));
                    draw.draw_glyph_run(Some(&ImmutableSolidColorBrush::new(Colors::BLACK)), &*run);
                    draw.pop_clip();
                    draw.dispose();
                    let pixels = bitmap.pixels();
                    assert!(pixels.chunks_exact(4).any(|p| p[0] < 100 && p[3] == 255));
                    assert_eq!(&pixels[(30 * 64 + 60) * 4..][..4], &[255, 255, 255, 255]);
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
}

#[test]
fn drawing_batch_retains_temporary_runs_and_balances_scopes_across_flushes() {
    assert_eq!(std::mem::size_of::<ffi::DrawCommand>(), 120);
    let face = face();
    let shaped = fonts::TextShaper.shape_text(
        &ReadOnlyMemory::from_str("A"),
        &TextShaperOptions::with_all(face.clone(), 14., 0, None, 0., 0., None),
    );
    let glyphs: Vec<_> = (0..shaped.length())
        .map(|i| shaped.glyph_infos()[i])
        .collect();
    let actual = Bitmap::new(PixelSize::new(32, 32), Vector::new(96., 96.));
    let expected = Bitmap::new(PixelSize::new(32, 32), Vector::new(96., 96.));
    let mut draw = actual.drawing(false, None);
    draw.clear(Colors::TRANSPARENT);
    for i in 0..1500 {
        let run = GlyphRun::new(
            &face,
            14.,
            &glyphs,
            Point::new((i % 12) as f64, 14. + (i % 10) as f64),
        );
        draw.push_clip(Rect::new(0., 0., 32., 32.));
        draw.draw_glyph_run(Some(&ImmutableSolidColorBrush::new(Colors::BLACK)), &run);
        draw.pop_clip();
        expected.context(|c| unsafe {
            ffi::d2_glyphs(
                c,
                run.font.raw(),
                run.size,
                run.ids.as_ptr(),
                run.positions.as_ptr(),
                run.ids.len(),
                0.,
                0.,
                0.,
                1.,
            );
        });
        // run drops here, before its queued native command necessarily executes.
    }
    draw.dispose();
    assert_eq!(actual.pixels(), expected.pixels());
}

#[test]
fn covering_integer_clips_stay_balanced_through_layers_snapshots_and_pop() {
    let bitmap = Bitmap::new(PixelSize::new(32, 32), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(false, None);
    draw.clear(Colors::TRANSPARENT);
    draw.push_clip(Rect::new(4., 4., 24., 24.));
    draw.push_clip(Rect::new(8., 8., 8., 8.));
    draw.push_clip(Rect::new(0., 0., 32., 32.));
    draw.push_opacity(0.5, None);
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::RED)),
        None,
        RoundedRect::from_rect(Rect::new(0., 0., 32., 32.)),
        &Default::default(),
    );
    draw.pop_opacity();
    let intermediate = bitmap.pixels();
    assert!(intermediate[(10 * 32 + 10) * 4 + 3] >= 127);
    assert_eq!(intermediate[(6 * 32 + 6) * 4 + 3], 0);
    draw.pop_clip();
    draw.pop_clip();
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::BLUE)),
        None,
        RoundedRect::from_rect(Rect::new(18., 18., 4., 4.)),
        &Default::default(),
    );
    draw.pop_clip();
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::GREEN)),
        None,
        RoundedRect::from_rect(Rect::new(0., 0., 2., 2.)),
        &Default::default(),
    );
    draw.dispose();
    let pixels = bitmap.pixels();
    assert_eq!(&pixels[(20 * 32 + 20) * 4..][..4], &[255, 0, 0, 255]);
    assert_eq!(pixels[(6 * 32 + 6) * 4 + 3], 0);
    assert_eq!(pixels[(1 * 32 + 1) * 4 + 3], 255);
}

#[test]
fn shaped_run_cache_keeps_owner_context_ranges_sizes_and_features_distinct() {
    use ferroui_base::media::FontFeature;
    let latin =
        include_bytes!("../../../samples/ControlCatalog/Assets/Fonts/SourceSansPro-Regular.ttf")
            .as_slice();
    let arabic = include_bytes!(
        "../../../samples/ControlCatalog/Assets/FallbackFonts/NotoSansArabic-Catalog.ttf"
    )
    .as_slice();
    for (data, cases) in [
        (
            latin,
            vec![
                ("office AV", 0, 9),
                ("office AV", 1, 2),
                ("office AV", 2, 2),
                ("affect AV", 1, 2),
            ],
        ),
        (
            arabic,
            vec![("بب", 0, 1), ("ب ", 0, 1), ("بب", 1, 1), (" ب", 1, 1)],
        ),
    ] {
        let manager = fonts::FontManager::default();
        let make_face = || {
            GlyphTypeface::new(
                manager
                    .try_create_glyph_typeface_from_stream(&mut &data[..], FontSimulations::None)
                    .unwrap(),
                FontSimulations::None,
            )
            .unwrap()
        };
        let cached_face = make_face();
        for _ in 0..2 {
            for (owner, offset, length) in &cases {
                for size in [12., 24.] {
                    for features in [
                        None,
                        Some(Rc::new(vec![FontFeature {
                            tag: "liga".into(),
                            value: 0,
                            start: 0,
                            end: 2,
                        }])),
                    ] {
                        let memory = ReadOnlyMemory::from_str(owner).slice(*offset, *length);
                        let options = |face| {
                            TextShaperOptions::with_all(
                                face,
                                size,
                                if data.len() == arabic.len() { 1 } else { 0 },
                                None,
                                40.,
                                1.25,
                                features.clone(),
                            )
                        };
                        let expected = fonts::TextShaper.shape_text(&memory, &options(make_face()));
                        let actual =
                            fonts::TextShaper.shape_text(&memory, &options(cached_face.clone()));
                        assert_eq!(actual.length(), expected.length());
                        for i in 0..actual.length() {
                            assert_eq!(
                                actual.glyph_infos()[i],
                                expected.glyph_infos()[i],
                                "Cached shape changed {owner:?} at {offset}/{length}, size {size}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn cached_positioned_glyphs_preserve_color_baselines_and_aliasing() {
    let manager = fonts::FontManager::default();
    for (character, family) in [('A', "Segoe UI"), ('😀', "Segoe UI Emoji")] {
        let platform = manager
            .try_create_glyph_typeface(
                family,
                FontStyle::Normal,
                FontWeight::Normal,
                FontStretch::Normal,
            )
            .unwrap();
        let face = GlyphTypeface::new(platform, FontSimulations::None).unwrap();
        let font = face
            .platform_typeface()
            .as_any()
            .downcast_ref::<fonts::Typeface>()
            .unwrap()
            .font
            .clone();
        let ids = [face.character_to_glyph_map().get_glyph(character as i32); 3];
        let positions = [7.25, 35., 100., 52.5, 52.5, 30.25];
        let cached = unsafe {
            ffi::Owned::new(ffi::d2_positioned_glyph_run(
                font.raw(),
                28.,
                ids.as_ptr(),
                positions.as_ptr(),
                ids.len(),
                std::ptr::null(),
            ))
        };
        for aliased in [0, 1] {
            let actual = Bitmap::new(PixelSize::new(160, 70), Vector::new(96., 96.));
            let expected = Bitmap::new(PixelSize::new(160, 70), Vector::new(96., 96.));
            let mut shifted = positions;
            for p in shifted.chunks_exact_mut(2) {
                p[1] += 0.25;
            }
            actual.context(|c| unsafe {
                ffi::d2_draw_positioned_glyph_run(c, cached.0, 0.25, aliased, 0., 0., 0., 1.);
            });
            expected.context(|c| unsafe {
                ffi::d2_gpu_render_options(c, 0, aliased, 1);
                ffi::d2_glyphs(
                    c,
                    font.raw(),
                    28.,
                    ids.as_ptr(),
                    shifted.as_ptr(),
                    ids.len(),
                    0.,
                    0.,
                    0.,
                    1.,
                );
            });
            let pixels = actual.pixels();
            assert_eq!(
                pixels,
                expected.pixels(),
                "Cached {character} glyphs changed baseline/antialiasing"
            );
            assert!(pixels.chunks_exact(4).any(|p| p[3] > 0));
            if character == '😀' {
                assert!(pixels
                    .chunks_exact(4)
                    .any(|p| p[3] > 0 && (p[0] != p[1] || p[1] != p[2])));
            }
        }
    }
}

#[test]
fn failed_drawing_session_recreates_its_target_at_the_next_boundary() {
    let bitmap = Bitmap::new(PixelSize::new(16, 16), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(false, None);
    draw.clear(Colors::RED);
    // An unbalanced native state makes finish mark the surface as lost.
    bitmap.context(|p| unsafe { ffi::d2_save(p) });
    draw.dispose();
    let mut draw = bitmap.drawing(false, None);
    draw.clear(Colors::BLUE);
    draw.dispose();
    assert!(bitmap
        .pixels()
        .chunks_exact(4)
        .all(|p| p == [255, 0, 0, 255]));
}

#[test]
fn rectangle_clips_match_geometry_through_mixed_stacks_and_snapshots() {
    fn render(rectangle: bool, transform: Matrix) -> Vec<u8> {
        let bitmap = Bitmap::new(PixelSize::new(96, 96), Vector::new(96., 96.));
        let mut draw = bitmap.drawing(false, None);
        draw.clear(Colors::TRANSPARENT);
        draw.set_transform(transform);
        let outer = Rect::new(2.25, 3.5, 48.5, 44.75);
        let inner = Rect::new(10.25, 8.5, 28.25, 26.5);
        if rectangle {
            draw.push_clip(outer);
        } else {
            draw.push_geometry_clip(&*Geometry::rectangle(outer));
        }
        draw.push_geometry_clip(&*Geometry::rectangle(Rect::new(7.5, 5.25, 34.5, 33.5)));
        if rectangle {
            draw.push_clip(inner);
        } else {
            draw.push_geometry_clip(&*Geometry::rectangle(inner));
        }
        draw.push_opacity(0.5, None);
        draw.draw_rectangle(
            Some(&ImmutableSolidColorBrush::new(Colors::RED)),
            None,
            RoundedRect::from_rect(Rect::new(-50., -50., 180., 180.)),
            &Default::default(),
        );
        draw.pop_opacity();
        // Snapshot suspends and restores all three clips in stack order.
        let first = bitmap.pixels();
        assert!(first.chunks_exact(4).any(|p| p[3] > 100));
        draw.draw_rectangle(
            Some(&ImmutableSolidColorBrush::new(Colors::BLUE)),
            None,
            RoundedRect::from_rect(Rect::new(12., 12., 12., 10.)),
            &Default::default(),
        );
        if rectangle {
            draw.pop_clip();
        } else {
            draw.pop_geometry_clip();
        }
        draw.pop_geometry_clip();
        if rectangle {
            draw.pop_clip();
        } else {
            draw.pop_geometry_clip();
        }
        draw.dispose();
        bitmap.pixels()
    }
    for transform in [
        Matrix::create_translation(10., 12.),
        Matrix::new(1.25, 0., 0., 1.5, 5.25, 7.5),
        Matrix::new(-1., 0., 0., 1., 80., 8.),
        Matrix::new(0.8, 0.4, -0.3, 0.9, 28., 9.),
    ] {
        let fast = render(true, transform);
        let geometry = render(false, transform);
        let max_difference = fast
            .iter()
            .zip(&geometry)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        // Direct2D's rectangle clip and geometry mask quantize fractional
        // antialias coverage slightly differently. Interiors must still match;
        // differences are confined to a one-pixel clip fringe.
        assert!(
            max_difference <= 16,
            "Clip coverage differs by {max_difference} at {transform:?}"
        );
        for (pixel, (a, b)) in fast
            .chunks_exact(4)
            .zip(geometry.chunks_exact(4))
            .enumerate()
        {
            if a.iter().zip(b).any(|(a, b)| a.abs_diff(*b) > 2) {
                assert_eq!((transform.m12, transform.m21), (0., 0.));
                let x = (pixel % 96) as f64 + 0.5;
                let y = (pixel / 96) as f64 + 0.5;
                assert!(
                    (x - (10.25 * transform.m11 + transform.m31)).abs() <= 1.
                        || (x - (38.5 * transform.m11 + transform.m31)).abs() <= 1.
                        || (y - (8.5 * transform.m22 + transform.m32)).abs() <= 1.
                        || (y - (35. * transform.m22 + transform.m32)).abs() <= 1.,
                    "Clipping changed an interior pixel at ({x}, {y})"
                );
            }
        }
    }
}

#[test]
fn glyph_outlines_keep_intersections_geometry_and_synthetic_bounds() {
    use kurbo::Shape;
    let manager = fonts::FontManager::default();
    let data =
        include_bytes!("../../../samples/ControlCatalog/Assets/Fonts/SourceSansPro-Regular.ttf");
    for simulations in [
        FontSimulations::None,
        FontSimulations::Bold,
        FontSimulations::Oblique,
    ] {
        let platform = manager
            .try_create_glyph_typeface_from_stream(&mut &data[..], simulations)
            .unwrap();
        let face = GlyphTypeface::new(platform, simulations).unwrap();
        let shaped = fonts::TextShaper.shape_text(
            &ReadOnlyMemory::from_str("Hello"),
            &TextShaperOptions::with_all(face.clone(), 24., 0, None, 0., 0., None),
        );
        let glyphs: Vec<_> = (0..shaped.length())
            .map(|i| shaped.glyph_infos()[i])
            .collect();
        let run = GlyphRun::new(&face, 24., &glyphs, Point::new(5., 30.));
        let bounds_before = run.bounds();
        let intersections = run.get_intersections(-8., -2.);
        assert!(!intersections.is_empty());
        assert_eq!(intersections.len() % 2, 0);
        let outline_bounds = run.outlines().bounding_box();
        assert!(outline_bounds.width() > 20. && outline_bounds.height() > 10.);
        assert_eq!(run.bounds(), bounds_before);
        if simulations != FontSimulations::None {
            assert_eq!(
                bounds_before,
                Rect::new(
                    outline_bounds.x0,
                    outline_bounds.y0,
                    outline_bounds.width(),
                    outline_bounds.height()
                )
            );
        }
        let bitmap = Bitmap::new(PixelSize::new(140, 50), Vector::new(96., 96.));
        let mut draw = bitmap.drawing(false, None);
        draw.clear(Colors::TRANSPARENT);
        draw.draw_glyph_run(Some(&ImmutableSolidColorBrush::new(Colors::BLACK)), &run);
        draw.dispose();
        assert!(bitmap.pixels().chunks_exact(4).filter(|p| p[3] > 0).count() > 100);
    }
}

#[test]
fn translated_clips_preserve_text_and_apply_the_transform_once() {
    let bitmap = Bitmap::new(PixelSize::new(100, 100), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(false, None);
    draw.set_transform(Matrix::create_translation(20., 30.));
    draw.push_clip(Rect::new(0., 0., 25., 25.));
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::RED)),
        None,
        RoundedRect::from_rect(Rect::new(-10., -10., 50., 50.)),
        &Default::default(),
    );
    draw.pop_clip();
    draw.dispose();
    let pixels = bitmap.pixels();
    assert_eq!(&pixels[(35 * 100 + 25) * 4..][..4], &[0, 0, 255, 255]);
    assert_eq!(&pixels[(65 * 100 + 45) * 4..][..4], &[0, 0, 0, 0]);
}
#[test]
fn immediate_xaml_brushes_use_fallback_color_for_fills_and_masks() {
    use ferroui_base::media::{IBrush, XamlCompositionBrushBase};
    let reveal = ferroui_controls::media::RevealBorderBrush::new();
    let xaml = reveal.clone().upcast::<XamlCompositionBrushBase>();
    xaml.set_fallback_color(Colors::RED);
    xaml.set_opacity(0.5);
    let brush: Rc<dyn IBrush> = reveal.into();
    for masked in [false, true] {
        let bitmap = Bitmap::new(PixelSize::new(4, 4), Vector::new(96., 96.));
        let mut draw = bitmap.drawing(false, None);
        let rect = Rect::new(0., 0., 4., 4.);
        if masked {
            draw.push_opacity_mask(&*brush, rect);
        }
        let blue = ImmutableSolidColorBrush::new(Colors::BLUE);
        draw.draw_rectangle(
            Some(if masked { &blue } else { &*brush }),
            None,
            RoundedRect::from_rect(rect),
            &Default::default(),
        );
        if masked {
            draw.pop_opacity_mask();
        }
        draw.dispose();
        let pixel = &bitmap.pixels()[..4];
        assert!(pixel[3].abs_diff(128) <= 1);
        assert!(pixel[if masked { 0 } else { 2 }].abs_diff(128) <= 1);
        assert_eq!(pixel[1], 0);
        assert_eq!(pixel[if masked { 2 } else { 0 }], 0);
    }
}

#[test]
fn compositor_and_immediate_rendering_agree_at_fractional_and_double_dpi() {
    for scale in [1., 1.25, 1.5, 2.] {
        let size = PixelSize::new(100, 100);
        let dpi = Vector::new(96. * scale, 96. * scale);
        let immediate = Bitmap::new(size, dpi);
        let composed = Bitmap::new(size, dpi);
        let paint = |draw: &mut dyn IDrawingContextImpl| {
            draw.push_clip(Rect::new(4., 5., 28., 24.));
            draw.draw_rectangle(
                Some(&ImmutableSolidColorBrush::new(Colors::RED)),
                None,
                RoundedRect::from_rect(Rect::new(0., 0., 40., 40.)),
                &Default::default(),
            );
            draw.pop_clip();
            draw.dispose();
        };
        paint(&mut immediate.drawing(true, None));
        let mut context = crate::presentation::compositor_drawing(&composed, None);
        context.set_transform(Matrix::create_scale(scale, scale));
        paint(&mut *context);
        assert_eq!(
            immediate.pixels(),
            composed.pixels(),
            "DPI scale {scale} was applied more than once"
        );
    }
}
fn face() -> Rc<GlyphTypeface> {
    let manager = fonts::FontManager::default();
    let platform = manager
        .try_create_glyph_typeface(
            "Segoe UI",
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
        )
        .unwrap();
    GlyphTypeface::new(platform, FontSimulations::None).unwrap()
}
#[test]
fn directwrite_shapes_utf16_subranges_and_rtl() {
    let shaper = fonts::TextShaper;
    let face = face();
    let owner = ReadOnlyMemory::from_str("prefix office suffix");
    let text = owner.slice(7, 6);
    let out = shaper.shape_text(
        &text,
        &TextShaperOptions::with_all(face.clone(), 20., 0, None, 0., 0., None),
    );
    assert!(out.length() > 0);
    for i in 0..out.length() {
        let g = out.glyph_infos()[i];
        assert!(g.glyph_cluster >= 0 && g.glyph_cluster < 6);
        assert!(g.glyph_advance >= 0.);
        assert_ne!(g.glyph_index, 0);
    }
    let text = ReadOnlyMemory::from_str("אבג");
    let manager = fonts::FontManager::default();
    let font = manager
        .try_match_character(
            'א' as i32,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            None,
            None,
        )
        .unwrap();
    let face = GlyphTypeface::new(font, FontSimulations::None).unwrap();
    let out = shaper.shape_text(
        &text,
        &TextShaperOptions::with_all(face, 20., 1, None, 0., 0., None),
    );
    assert_eq!(out.length(), 3);
    assert_eq!(
        (
            out.glyph_infos()[0].glyph_cluster,
            out.glyph_infos()[2].glyph_cluster
        ),
        (2, 0)
    );
    assert!((0..out.length()).all(|i| out.glyph_infos()[i].glyph_index != 0));
}

#[test]
fn large_text_buffers_shape_paragraphs_without_losing_slice_clusters() {
    let options = TextShaperOptions::with_all(face(), 20., 0, None, 0., 0., None);
    let line = "office sample paragraph office\n";
    let source = line.repeat(3000);
    let text = ReadOnlyMemory::from_str(&source);
    assert!(text.len() > 65535);
    let start = line.len() * 2500;
    let slice = fonts::TextShaper.shape_text(&text.slice(start, 6), &options);
    let isolated = fonts::TextShaper.shape_text(&ReadOnlyMemory::from_str("office"), &options);
    assert_eq!(slice.length(), isolated.length());
    for i in 0..slice.length() {
        assert_eq!(slice.glyph_infos()[i], isolated.glyph_infos()[i]);
    }
    let whole = fonts::TextShaper.shape_text(&text, &options);
    assert!(whole.length() > 60000);
    for i in 0..whole.length() {
        let glyph = whole.glyph_infos()[i];
        assert_ne!(glyph.glyph_index, 0);
        if text.span()[glyph.glyph_cluster as usize] == 10 {
            assert_eq!(glyph.glyph_advance, 0.);
        }
    }
}

#[test]
fn formatter_slices_do_not_lose_characters_to_context_ligatures() {
    let manager = fonts::FontManager::default();
    let data =
        include_bytes!("../../../samples/ControlCatalog/Assets/Fonts/SourceSansPro-Regular.ttf");
    let platform = manager
        .try_create_glyph_typeface_from_stream(&mut &data[..], FontSimulations::None)
        .unwrap();
    let options = TextShaperOptions::with_all(
        GlyphTypeface::new(platform, FontSimulations::None).unwrap(),
        20.,
        0,
        None,
        0.,
        0.,
        None,
    );
    let text = ReadOnlyMemory::from_str("office");
    let whole = fonts::TextShaper.shape_text(&text, &options);
    assert!(
        whole.length() < text.len(),
        "The fixture must contain a ligature"
    );
    let reference = fonts::TextShaper.shape_text(&ReadOnlyMemory::from_str("f"), &options);
    for start in [1, 2] {
        let slice = fonts::TextShaper.shape_text(&text.slice(start, 1), &options);
        assert_eq!(slice.length(), reference.length());
        assert_eq!(slice.glyph_infos()[0], reference.glyph_infos()[0]);
    }
}

#[test]
fn opentype_feature_ranges_preserve_defaults_and_owner_coordinates() {
    use ferroui_base::media::FontFeature;
    let manager = fonts::FontManager::default();
    let data =
        include_bytes!("../../../samples/ControlCatalog/Assets/Fonts/SourceSansPro-Regular.ttf");
    let platform = manager
        .try_create_glyph_typeface_from_stream(&mut &data[..], FontSimulations::None)
        .unwrap();
    let face = GlyphTypeface::new(platform, FontSimulations::None).unwrap();
    let text = ReadOnlyMemory::from_str("ff ff");
    let options =
        |features| TextShaperOptions::with_all(face.clone(), 20., 0, None, 0., 0., features);
    let standard = fonts::TextShaper.shape_text(&text, &options(None));
    assert_eq!(standard.length(), 3);
    let no_kern = Rc::new(vec![FontFeature {
        tag: "kern".into(),
        value: 0,
        ..Default::default()
    }]);
    assert_eq!(
        fonts::TextShaper
            .shape_text(&text, &options(Some(no_kern)))
            .length(),
        3
    );
    for (start, end, clusters) in [(0, 2, vec![0, 1, 2, 3]), (3, 5, vec![0, 2, 3, 4])] {
        let features = Rc::new(vec![FontFeature {
            tag: "liga".into(),
            value: 0,
            start,
            end,
        }]);
        let shaped = fonts::TextShaper.shape_text(&text, &options(Some(features.clone())));
        assert_eq!(shaped.length(), clusters.len());
        for (i, cluster) in clusters.into_iter().enumerate() {
            assert_eq!(shaped.glyph_infos()[i].glyph_cluster, cluster);
        }
        let slice = fonts::TextShaper.shape_text(&text.slice(3, 2), &options(Some(features)));
        assert_eq!(slice.length(), if start == 3 { 2 } else { 1 });
        assert_eq!(slice.glyph_infos()[0].glyph_cluster, 0);
    }
}

#[test]
fn long_script_runs_preserve_ligatures_combining_marks_and_slice_clusters() {
    use ferroui_base::media::FontFeature;
    let manager = fonts::FontManager::default();
    let data =
        include_bytes!("../../../samples/ControlCatalog/Assets/Fonts/SourceSansPro-Regular.ttf");
    let platform = manager
        .try_create_glyph_typeface_from_stream(&mut &data[..], FontSimulations::None)
        .unwrap();
    let face = GlyphTypeface::new(platform, FontSimulations::None).unwrap();
    let features = Rc::new(vec![FontFeature {
        tag: "liga".into(),
        value: 1,
        ..Default::default()
    }]);
    let options = TextShaperOptions::with_all(face, 20., 0, None, 0., 0., Some(features));
    let word = "office AV a\u{301} ";
    let units = word.encode_utf16().count();
    let reference = fonts::TextShaper.shape_text(&ReadOnlyMemory::from_str(word), &options);
    assert!(
        reference.length() < units,
        "The fixture must exercise a ligature or combining cluster"
    );
    let text = ReadOnlyMemory::from_str(&word.repeat(7000));
    let whole = fonts::TextShaper.shape_text(&text, &options);
    assert_eq!(whole.length(), reference.length() * 7000);
    for i in 0..whole.length() {
        let actual = whole.glyph_infos()[i];
        let expected = reference.glyph_infos()[i % reference.length()];
        assert_eq!(actual.glyph_index, expected.glyph_index);
        assert_eq!(
            actual.glyph_cluster,
            expected.glyph_cluster + (i / reference.length() * units) as i32
        );
        assert!((actual.glyph_advance - expected.glyph_advance).abs() < 0.001);
        assert_eq!(actual.glyph_offset, expected.glyph_offset);
    }
    let slice = fonts::TextShaper.shape_text(&text.slice(units * 3500, units), &options);
    assert_eq!(slice.length(), reference.length());
    for i in 0..slice.length() {
        assert_eq!(slice.glyph_infos()[i], reference.glyph_infos()[i]);
    }
}

#[test]
fn long_rtl_paragraphs_keep_joining_and_global_cluster_order() {
    let manager = fonts::FontManager::default();
    let data = include_bytes!(
        "../../../samples/ControlCatalog/Assets/FallbackFonts/NotoSansArabic-Catalog.ttf"
    );
    let platform = manager
        .try_create_glyph_typeface_from_stream(&mut &data[..], FontSimulations::None)
        .unwrap();
    let face = GlyphTypeface::new(platform, FontSimulations::None).unwrap();
    let options = TextShaperOptions::with_all(face, 20., 1, None, 0., 0., None);
    let word = "سلام ";
    let units = word.encode_utf16().count();
    let reference = fonts::TextShaper.shape_text(&ReadOnlyMemory::from_str(word), &options);
    let text = ReadOnlyMemory::from_str(&word.repeat(20000));
    let whole = fonts::TextShaper.shape_text(&text, &options);
    assert_eq!(whole.length(), reference.length() * 20000);
    for i in 0..whole.length() {
        let actual = whole.glyph_infos()[i];
        let expected = reference.glyph_infos()[i % reference.length()];
        assert_eq!(actual.glyph_index, expected.glyph_index);
        assert_eq!(
            actual.glyph_cluster,
            expected.glyph_cluster + ((19999 - i / reference.length()) * units) as i32
        );
        assert!((actual.glyph_advance - expected.glyph_advance).abs() < 0.001);
        assert_eq!(actual.glyph_offset, expected.glyph_offset);
    }
}

#[test]
fn long_emoji_runs_keep_surrogate_clusters() {
    let manager = fonts::FontManager::default();
    let platform = manager
        .try_create_glyph_typeface(
            "Segoe UI Emoji",
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
        )
        .unwrap();
    let face = GlyphTypeface::new(platform, FontSimulations::None).unwrap();
    let options = TextShaperOptions::with_all(face, 20., 0, None, 0., 0., None);
    let text = ReadOnlyMemory::from_str(&"😀".repeat(20000));
    let whole = fonts::TextShaper.shape_text(&text, &options);
    assert_eq!(whole.length(), 20000);
    for i in 0..whole.length() {
        assert_ne!(whole.glyph_infos()[i].glyph_index, 0);
        assert_eq!(whole.glyph_infos()[i].glyph_cluster, (2 * i) as i32);
    }
}

#[test]
fn embedded_fonts_survive_the_loader_and_keep_arabic_slice_context() {
    let manager = fonts::FontManager::default();
    let data = include_bytes!("../../FerroUI.Base/test_assets/fonts/Inter-Regular.ttf");
    let platform = manager
        .try_create_glyph_typeface_from_stream(&mut &data[..], FontSimulations::None)
        .unwrap();
    drop(manager);
    let embedded = GlyphTypeface::new(platform, FontSimulations::None).unwrap();
    let text = ReadOnlyMemory::from_str("office");
    let out = fonts::TextShaper.shape_text(
        &text,
        &TextShaperOptions::with_all(embedded, 20., 0, None, 0., 0., None),
    );
    assert!(out.length() > 0);
    assert!((0..out.length()).all(|i| out.glyph_infos()[i].glyph_index != 0));

    let manager = fonts::FontManager::default();
    let data = include_bytes!(
        "../../../samples/ControlCatalog/Assets/FallbackFonts/NotoSansArabic-Catalog.ttf"
    );
    let platform = manager
        .try_create_glyph_typeface_from_stream(&mut &data[..], FontSimulations::None)
        .unwrap();
    let face = GlyphTypeface::new(platform, FontSimulations::None).unwrap();
    let text = ReadOnlyMemory::from_str("سلامسلام");
    let options = TextShaperOptions::with_all(face, 24., 1, None, 0., 0., None);
    let whole = fonts::TextShaper.shape_text(&text, &options);
    let slice = fonts::TextShaper.shape_text(&text.slice(4, 4), &options);
    let expected: Vec<_> = (0..whole.length())
        .map(|i| whole.glyph_infos()[i])
        .filter(|g| g.glyph_cluster >= 4)
        .collect();
    assert_eq!(slice.length(), expected.len());
    for (i, g) in expected.into_iter().enumerate() {
        let actual = slice.glyph_infos()[i];
        assert_ne!(actual.glyph_index, 0);
        assert_eq!(actual.glyph_index, g.glyph_index);
        assert_eq!(actual.glyph_cluster, g.glyph_cluster - 4);
        assert!((actual.glyph_advance - g.glyph_advance).abs() < 1e-6);
    }
}

#[test]
fn variable_font_axes_and_surrogate_clusters_use_directwrite() {
    use ferroui_base::media::fonts::OpenTypeTag;
    let manager = fonts::FontManager::default();
    assert!(manager
        .try_create_glyph_typeface(
            "FerroUI deliberate missing font family",
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal
        )
        .is_none());
    let data = include_bytes!("../../FerroUI.Base/test_assets/fonts/InterVariable.ttf");
    let tag = OpenTypeTag::parse("wght");
    for weight in [100., 900.] {
        let face = manager
            .create_variable_typeface_from_stream(
                &mut &data[..],
                FontSimulations::None,
                &[(tag, weight)],
            )
            .unwrap();
        let face = GlyphTypeface::new(face, FontSimulations::None).unwrap();
        let out = fonts::TextShaper.shape_text(
            &ReadOnlyMemory::from_str("Windows"),
            &TextShaperOptions::with_all(face, 24., 0, None, 0., 0., None),
        );
        assert!(out.length() > 0);
        assert!((0..out.length()).all(|i| out.glyph_infos()[i].glyph_index != 0));
    }
    assert!(manager
        .create_variable_typeface_from_stream(
            &mut &data[..],
            FontSimulations::None,
            &[(tag, 10000.)]
        )
        .is_none());
    let emoji = manager
        .try_match_character(
            0x1f600,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            None,
            None,
        )
        .unwrap();
    let emoji = GlyphTypeface::new(emoji, FontSimulations::None).unwrap();
    let out = fonts::TextShaper.shape_text(
        &ReadOnlyMemory::from_str("😀😀"),
        &TextShaperOptions::with_all(emoji, 24., 0, None, 0., 0., None),
    );
    assert_eq!(out.length(), 2);
    assert_eq!(
        (
            out.glyph_infos()[0].glyph_cluster,
            out.glyph_infos()[1].glyph_cluster
        ),
        (0, 2)
    );
    assert!((0..out.length()).all(|i| out.glyph_infos()[i].glyph_index != 0));
}
#[test]
fn system_geometry_boolean_operations_preserve_holes() {
    let a = Geometry::rectangle(Rect::new(0., 0., 20., 20.));
    let zero_pen =
        ferroui_base::media::immutable::ImmutablePen::from_uint32(Colors::BLACK.to_uint32(), 0.);
    assert_eq!(a.get_render_bounds(Some(&zero_pen)), a.bounds());
    let b = Geometry::rectangle(Rect::new(5., 5., 10., 10.));
    let difference = Geometry::combine(&*a, &*b, GeometryCombineMode::Exclude);
    assert!(difference.fill_contains(Point::new(2., 2.)));
    assert!(!difference.fill_contains(Point::new(10., 10.)));
    let intersection = Geometry::combine(&*a, &*b, GeometryCombineMode::Intersect);
    assert!(!intersection.fill_contains(Point::new(2., 2.)));
    assert!(intersection.fill_contains(Point::new(10., 10.)));
}

#[test]
fn parity_geometry_distinguishes_containment_overlap_and_disjoint_fills() {
    use ferroui_base::media::IntersectionResult;
    let outer = Geometry::rectangle(Rect::new(0., 0., 40., 40.));
    let inner = Geometry::rectangle(Rect::new(10., 10., 10., 10.));
    let overlap = Geometry::rectangle(Rect::new(30., 30., 20., 20.));
    let disjoint = Geometry::rectangle(Rect::new(60., 60., 10., 10.));
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
        outer.get_fill_intersection_result(&*disjoint),
        IntersectionResult::Empty
    );
    let ring = Geometry::combine(
        &*outer,
        &*inner,
        ferroui_base::media::GeometryCombineMode::Exclude,
    );
    assert_eq!(
        ring.get_fill_intersection_result(&*inner),
        IntersectionResult::Empty,
        "{:?}",
        Geometry::combine(
            &*ring,
            &*inner,
            ferroui_base::media::GeometryCombineMode::Intersect
        )
        .fill
    );
}

#[test]
fn parity_geometry_render_bounds_include_unstroked_fill_curves() {
    use ferroui_base::platform::IGeometryContext;
    let mut builder = geometry::Builder::new(None);
    builder.begin_figure(Point::new(0., 0.), true);
    builder.line_to(Point::new(20., 0.), true);
    builder.quadratic_bezier_to(Point::new(150., 100.), Point::new(20., 40.), false);
    builder.end_figure(true);
    let shape = builder.to_geometry();
    let pen = ferroui_base::media::immutable::ImmutablePen::from_uint32(0xff000000, 2.);
    assert!(shape.get_render_bounds(None).right() > 70.);
    assert!(shape.get_render_bounds(Some(&pen)).right() > 70.);
}

#[test]
fn retained_widened_strokes_match_the_exported_path_under_clip() {
    use ferroui_base::media::{
        immutable::{ImmutableDashStyle, ImmutablePen},
        PenLineCap, PenLineJoin,
    };
    use std::rc::Rc;

    let mut stroke = kurbo::BezPath::new();
    stroke.move_to((4., 11.));
    stroke.curve_to((33., 3.), (12., 38.), (35., 31.));
    stroke.line_to((4., 27.));
    stroke.line_to((32., 9.));
    let geometry = geometry::Geometry::paths(stroke.clone(), None, FillRule::NonZero);
    let paint = Rc::new(ImmutableSolidColorBrush::from_uint32(0xbfc06010));
    for dash in [
        None,
        Some(Rc::new(ImmutableDashStyle::new(Some(&[2., 1.5]), 0.5))),
    ] {
        let pen = ImmutablePen::new(
            Some(paint.clone()),
            3.25,
            dash,
            PenLineCap::Round,
            PenLineJoin::Round,
            10.,
        );
        let reference = geometry::Geometry::new(
            geometry::Geometry::stroked(&stroke, &pen),
            FillRule::NonZero,
        );
        let render = |widened| {
            let bitmap = Bitmap::new(PixelSize::new(64, 64), Vector::new(96., 96.));
            let mut draw = bitmap.drawing(false, None);
            draw.clear(Colors::WHITE);
            draw.push_clip(Rect::new(8.5, 5.25, 34.25, 39.5));
            draw.set_transform(
                Matrix::create_scale(1.25, 1.25) * Matrix::create_translation(0.25, 0.5),
            );
            if widened {
                draw.draw_geometry(None, Some(&pen), &*geometry);
            } else {
                draw.draw_geometry(Some(&*paint), None, &*reference);
            }
            draw.pop_clip();
            draw.dispose();
            bitmap.pixels()
        };
        let direct = render(true);
        assert_eq!(direct, render(false));
        assert!(direct.chunks_exact(4).any(|pixel| pixel != [255; 4]));
    }
}

#[test]
fn parity_geometry_closes_broken_strokes_to_the_figure_start() {
    use ferroui_base::platform::IGeometryContext;
    let mut builder = geometry::Builder::new(None);
    builder.begin_figure(Point::new(0., 0.), true);
    builder.line_to(Point::new(80., 0.), true);
    builder.line_to(Point::new(80., 80.), false);
    builder.line_to(Point::new(0., 80.), true);
    builder.end_figure(true);
    let shape = builder.to_geometry();
    let pen = ferroui_base::media::immutable::ImmutablePen::from_uint32(0xff000000, 2.);
    assert!(shape.stroke_contains(Some(&pen), Point::new(0., 40.)));
    assert!(!shape.stroke_contains(Some(&pen), Point::new(80., 40.)));
}

#[test]
fn parity_writable_inputs_keep_declared_bytes_and_skip_unchanged_uploads() {
    let renderer = Direct2DRenderInterface;
    for format in [
        PixelFormat::RGBA8888,
        PixelFormat::BGRA8888,
        PixelFormat::RGB565,
    ] {
        for alpha in [
            AlphaFormat::Premul,
            AlphaFormat::Unpremul,
            AlphaFormat::Opaque,
        ] {
            assert!(renderer.is_supported_bitmap_pixel_format(format));
            let bitmap = renderer.create_writeable_bitmap(
                PixelSize::new(2, 1),
                Vector::new(96., 96.),
                format,
                alpha,
            );
            let actual_alpha = if format.has_alpha() {
                alpha
            } else {
                AlphaFormat::Opaque
            };
            assert_eq!(bitmap.alpha_format(), Some(actual_alpha));
            assert_eq!(bitmap.format(), Some(format));
            let lock = bitmap.lock();
            let mut expected = vec![0; lock.row_bytes() as usize];
            if format == PixelFormat::RGB565 {
                expected[..4].copy_from_slice(&[0, 248, 224, 7]);
            } else {
                let red = if format == PixelFormat::RGBA8888 {
                    0
                } else {
                    2
                };
                expected[red] = if alpha == AlphaFormat::Premul { 1 } else { 251 };
                expected[3] = 1;
                expected[5] = 255;
                expected[7] = 255;
            }
            lock.with_data(&mut |bytes| bytes.copy_from_slice(&expected));
            lock.dispose();
            let version = bitmap.version();
            let transfers = statistics();
            let lock = bitmap.lock();
            assert_eq!(lock.format(), format);
            assert_eq!(lock.alpha_format(), actual_alpha);
            lock.with_data(&mut |bytes| assert_eq!(bytes, expected));
            lock.dispose();
            assert_eq!(bitmap.version(), version);
            let after = statistics();
            assert_eq!(
                after[4], transfers[4],
                "canonical locks must not read GPU pixels"
            );
            assert_eq!(
                after[5], transfers[5],
                "unchanged locks must not upload pixels"
            );
            let target = Bitmap::new(PixelSize::new(2, 1), Vector::new(96., 96.));
            let mut drawing = target.drawing(false, None);
            drawing.draw_bitmap(
                &*bitmap,
                1.,
                Rect::new(0., 0., 2., 1.),
                Rect::new(0., 0., 2., 1.),
            );
            drawing.dispose();
            assert_eq!(target.pixels()[7], 255);
        }
    }
}
#[test]
fn clipping_opacity_and_snapshot_are_independent() {
    let bitmap = Bitmap::new(PixelSize::new(20, 20), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(false, None);
    draw.clear(Colors::WHITE);
    draw.push_clip(Rect::new(0., 0., 10., 20.));
    draw.push_opacity(0.5, None);
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::RED)),
        None,
        RoundedRect::from_rect(Rect::new(0., 0., 20., 20.)),
        &Default::default(),
    );
    draw.pop_opacity();
    draw.pop_clip();
    draw.dispose();
    let pixels = bitmap.pixels();
    let left = &pixels[(5 * 20 + 5) * 4..][..4];
    let right = &pixels[(5 * 20 + 15) * 4..][..4];
    assert!(left[0] > 110 && left[0] < 145 && left[2] == 255);
    assert_eq!(right, &[255, 255, 255, 255]);
    let snapshot = IDrawingContextLayerImpl::create_shared_snapshot(&bitmap);
    let mut draw = bitmap.drawing(false, None);
    draw.clear(Colors::BLUE);
    draw.dispose();
    let snapshot = snapshot.as_any().downcast_ref::<Bitmap>().unwrap();
    assert_eq!(snapshot.pixels(), pixels);
}
#[test]
fn wic_roundtrips_png_and_dpi_rendering_scales() {
    let bitmap = Bitmap::new(PixelSize::new(40, 40), Vector::new(192., 192.));
    let mut draw = bitmap.drawing(true, None);
    draw.clear(Colors::TRANSPARENT);
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::RED)),
        None,
        RoundedRect::from_rect(Rect::new(0., 0., 10., 10.)),
        &Default::default(),
    );
    draw.dispose();
    let pixels = bitmap.pixels();
    assert_eq!(pixels[(15 * 40 + 15) * 4 + 2], 255);
    assert_eq!(pixels[(25 * 40 + 25) * 4 + 3], 0);
    let mut png = Vec::new();
    bitmap
        .save(
            &mut png,
            &ferroui_base::media::imaging::PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let decoded = Bitmap::decode(&mut &png[..]).unwrap();
    assert_eq!(decoded.pixel_size(), PixelSize::new(40, 40));
    assert_eq!(decoded.pixels(), pixels);
}
#[test]
fn directwrite_glyphs_draw_visible_pixels() {
    let face = face();
    let text = ReadOnlyMemory::from_str("Hello Ferro");
    let options = TextShaperOptions::with_all(face.clone(), 24., 0, None, 0., 0., None);
    let shaped = fonts::TextShaper.shape_text(&text, &options);
    let glyphs: Vec<_> = (0..shaped.length())
        .map(|i| shaped.glyph_infos()[i])
        .collect();
    let run = GlyphRun::new(&face, 24., &glyphs, Point::new(5., 30.));
    let bitmap = Bitmap::new(PixelSize::new(180, 45), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(false, None);
    draw.clear(Colors::WHITE);
    draw.draw_glyph_run(Some(&ImmutableSolidColorBrush::new(Colors::BLACK)), &run);
    draw.dispose();
    assert!(
        bitmap
            .pixels()
            .chunks_exact(4)
            .filter(|p| p[0] < 100 && p[3] == 255)
            .count()
            > 100
    );
}

#[test]
fn positioned_glyph_batches_match_individual_draws_and_keep_color_glyphs() {
    let manager = fonts::FontManager::default();
    for (character, name) in [('A', "Segoe UI"), ('😀', "Segoe UI Emoji")] {
        let platform = manager
            .try_create_glyph_typeface(
                name,
                FontStyle::Normal,
                FontWeight::Normal,
                FontStretch::Normal,
            )
            .unwrap();
        let face = GlyphTypeface::new(platform, FontSimulations::None).unwrap();
        let id = face.character_to_glyph_map().get_glyph(character as i32);
        assert_ne!(id, 0);
        let font = face
            .platform_typeface()
            .as_any()
            .downcast_ref::<fonts::Typeface>()
            .unwrap()
            .font
            .clone();
        let batched = Bitmap::new(PixelSize::new(160, 70), Vector::new(96., 96.));
        let individual = Bitmap::new(PixelSize::new(160, 70), Vector::new(96., 96.));
        let ids = [id; 3];
        let positions = [7.25, 35., 100., 52.5, 52.5, 30.25];
        batched.context(|c| unsafe {
            ffi::d2_glyphs(
                c,
                font.raw(),
                28.,
                ids.as_ptr(),
                positions.as_ptr(),
                3,
                0.,
                0.,
                0.,
                1.,
            );
        });
        individual.context(|c| unsafe {
            for i in 0..3 {
                ffi::d2_glyphs(
                    c,
                    font.raw(),
                    28.,
                    ids[i..].as_ptr(),
                    positions[i * 2..].as_ptr(),
                    1,
                    0.,
                    0.,
                    0.,
                    1.,
                );
            }
        });
        let pixels = batched.pixels();
        assert_eq!(
            pixels,
            individual.pixels(),
            "positioned {character} glyphs changed when batched"
        );
        assert!(pixels.chunks_exact(4).any(|p| p[3] > 0));
        if character == '😀' {
            assert!(
                pixels
                    .chunks_exact(4)
                    .any(|p| p[3] > 0 && (p[0] != p[1] || p[1] != p[2])),
                "emoji lost its color layers"
            );
        }
    }
}

/// One premultiplied BGRA pixel of a bitmap that is `width` pixels wide.
fn pixel_at(pixels: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    pixels[(y * width + x) * 4..][..4].try_into().unwrap()
}

#[test]
fn bitmap_blending_mode_leaves_shapes_source_over() {
    use ferroui_base::media::{imaging::BitmapBlendingMode, RenderOptions};
    let bitmap = Bitmap::new(PixelSize::new(20, 20), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(false, None);
    draw.clear(Colors::WHITE);
    draw.push_render_options(RenderOptions {
        bitmap_blending_mode: BitmapBlendingMode::Source,
        ..Default::default()
    });
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::from_uint32(0x80ff0000)),
        None,
        RoundedRect::from_rect(Rect::new(0., 0., 10., 20.)),
        &Default::default(),
    );
    draw.pop_render_options();
    draw.dispose();
    let pixels = bitmap.pixels();
    // Half-transparent red over white stays opaque. A copy would have replaced the white.
    let left = pixel_at(&pixels, 20, 5, 10);
    assert!(
        left[0] > 110 && left[0] < 145 && left[2] >= 254 && left[3] == 255,
        "{left:?}"
    );
    assert_eq!(pixel_at(&pixels, 20, 15, 10), [255, 255, 255, 255]);
}

#[test]
fn bitmap_blending_modes_compose_bitmaps_and_do_not_leak_into_later_shapes() {
    use ferroui_base::media::{imaging::BitmapBlendingMode, Color, RenderOptions};
    fn solid(color: Color) -> Bitmap {
        let bitmap = Bitmap::new(PixelSize::new(8, 8), Vector::new(96., 96.));
        let mut draw = bitmap.drawing(false, None);
        draw.clear(color);
        draw.dispose();
        bitmap
    }
    // The bitmap covers the left half. A half-transparent red square follows it in the
    // bottom right quarter while the blending mode is still set.
    fn render(source: &Bitmap, mode: BitmapBlendingMode, background: Color) -> Vec<u8> {
        let target = Bitmap::new(PixelSize::new(20, 20), Vector::new(96., 96.));
        let mut draw = target.drawing(false, None);
        draw.clear(background);
        draw.push_render_options(RenderOptions {
            bitmap_blending_mode: mode,
            ..Default::default()
        });
        draw.draw_bitmap(source, 1., Rect::new(0., 0., 8., 8.), Rect::new(0., 0., 10., 20.));
        draw.draw_rectangle(
            Some(&ImmutableSolidColorBrush::from_uint32(0x80ff0000)),
            None,
            RoundedRect::from_rect(Rect::new(10., 10., 10., 10.)),
            &Default::default(),
        );
        draw.pop_render_options();
        draw.dispose();
        target.pixels()
    }
    let red = solid(Colors::RED);
    let clear = solid(Colors::TRANSPARENT);

    // Source replaces the destination, also with transparent pixels.
    let pixels = render(&clear, BitmapBlendingMode::Source, Colors::WHITE);
    assert_eq!(pixel_at(&pixels, 20, 5, 5), [0, 0, 0, 0]);
    assert_eq!(pixel_at(&pixels, 20, 15, 5), [255, 255, 255, 255]);
    let square = pixel_at(&pixels, 20, 15, 15);
    assert!(
        square[0] > 110 && square[0] < 145 && square[2] >= 254 && square[3] == 255,
        "{square:?}"
    );

    // Destination keeps what is there.
    let pixels = render(&red, BitmapBlendingMode::Destination, Colors::WHITE);
    assert_eq!(pixel_at(&pixels, 20, 5, 5), [255, 255, 255, 255]);
    assert_eq!(pixel_at(&pixels, 20, 15, 5), [255, 255, 255, 255]);

    // Plus adds the channels.
    let pixels = render(&red, BitmapBlendingMode::Plus, Colors::BLUE);
    assert_eq!(pixel_at(&pixels, 20, 5, 5), [255, 0, 255, 255]);
    assert_eq!(pixel_at(&pixels, 20, 15, 5), [255, 0, 0, 255]);

    // Multiply is composed through a layer: red times blue is black.
    let pixels = render(&red, BitmapBlendingMode::Multiply, Colors::BLUE);
    assert_eq!(pixel_at(&pixels, 20, 5, 5), [0, 0, 0, 255]);
    assert_eq!(pixel_at(&pixels, 20, 15, 5), [255, 0, 0, 255]);
    let square = pixel_at(&pixels, 20, 15, 15);
    assert!(
        square[0] > 110 && square[0] < 145 && square[2] > 110 && square[2] < 145,
        "{square:?}"
    );
    assert_eq!(square[3], 255);

    // SourceIn keeps the bitmap where the destination is opaque and ends at the bitmap edge.
    let pixels = render(&red, BitmapBlendingMode::SourceIn, Colors::BLUE);
    assert_eq!(pixel_at(&pixels, 20, 5, 5), [0, 0, 255, 255]);
    assert_eq!(pixel_at(&pixels, 20, 15, 5), [255, 0, 0, 255]);
}

#[test]
fn oversized_corner_radii_shrink_together() {
    let bitmap = Bitmap::new(PixelSize::new(80, 40), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(false, None);
    draw.clear(Colors::TRANSPARENT);
    // The left side has room for a third of the top left radius, so every radius is scaled
    // by a third: 40 at the top left and 13.33 at the top right.
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::RED)),
        None,
        RoundedRect::new(
            Rect::new(0., 0., 80., 40.),
            Vector::new(120., 120.),
            Vector::new(40., 40.),
            Vector::new(0., 0.),
            Vector::new(0., 0.),
        ),
        &Default::default(),
    );
    draw.dispose();
    let pixels = bitmap.pixels();
    let red = [0, 0, 255, 255];
    assert_eq!(pixel_at(&pixels, 80, 6, 6), [0; 4]);
    assert_eq!(pixel_at(&pixels, 80, 20, 20), red);
    // Inside the small top right corner. A corner limited on its own to half of each side
    // would leave this pixel empty.
    assert_eq!(pixel_at(&pixels, 80, 74, 5), red);
    assert_eq!(pixel_at(&pixels, 80, 78, 1), [0; 4]);
    assert_eq!(pixel_at(&pixels, 80, 0, 39), red);
    assert_eq!(pixel_at(&pixels, 80, 79, 39), red);
}

#[test]
fn clip_to_a_geometry_without_area_leaves_content_visible() {
    let bitmap = Bitmap::new(PixelSize::new(20, 20), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(false, None);
    draw.clear(Colors::TRANSPARENT);
    let line =
        Direct2DRenderInterface.create_line_geometry(Point::new(2., 2.), Point::new(18., 18.));
    draw.push_geometry_clip(&*line);
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::RED)),
        None,
        RoundedRect::from_rect(Rect::new(0., 0., 20., 20.)),
        &Default::default(),
    );
    draw.pop_geometry_clip();
    draw.dispose();
    // An unbalanced clip would have failed the session and left a blank target.
    assert!(bitmap
        .pixels()
        .chunks_exact(4)
        .all(|p| p == [0, 0, 255, 255]));
}

#[test]
fn pen_without_thickness_draws_nothing() {
    use ferroui_base::media::immutable::ImmutablePen;
    let bitmap = Bitmap::new(PixelSize::new(20, 20), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(false, None);
    draw.clear(Colors::TRANSPARENT);
    let pen = ImmutablePen::from_uint32(Colors::BLACK.to_uint32(), 0.);
    draw.draw_line(Some(&pen), Point::new(2., 2.), Point::new(18., 18.));
    draw.draw_rectangle(
        None,
        Some(&pen),
        RoundedRect::from_rect(Rect::new(4., 4., 12., 12.)),
        &Default::default(),
    );
    draw.draw_geometry(
        None,
        Some(&pen),
        &*geometry::Geometry::ellipse(Rect::new(2., 2., 16., 16.)),
    );
    draw.dispose();
    assert!(bitmap.pixels().iter().all(|&value| value == 0));
}

#[test]
fn relative_pen_brush_spans_the_outline() {
    use ferroui_base::media::{
        immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutablePen},
        GradientSpreadMethod,
    };
    use ferroui_base::{RelativePoint, RelativeUnit};
    // Red up to a quarter of the width, blue after it.
    let brush = ImmutableLinearGradientBrush::new(
        &[
            ImmutableGradientStop::new(0., Colors::RED),
            ImmutableGradientStop::new(0.24, Colors::RED),
            ImmutableGradientStop::new(0.26, Colors::BLUE),
            ImmutableGradientStop::new(1., Colors::BLUE),
        ],
        1.,
        None,
        None,
        GradientSpreadMethod::Pad,
        Some(RelativePoint::new(0., 0., RelativeUnit::Relative)),
        Some(RelativePoint::new(1., 0., RelativeUnit::Relative)),
        None,
    );
    let pen = ImmutablePen::with_brush(Some(Rc::new(brush)), 20.);
    let bitmap = Bitmap::new(PixelSize::new(60, 60), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(false, None);
    draw.clear(Colors::TRANSPARENT);
    draw.draw_rectangle(
        None,
        Some(&pen),
        RoundedRect::from_rect(Rect::new(20., 20., 20., 20.)),
        &Default::default(),
    );
    draw.dispose();
    let pixels = bitmap.pixels();
    // The outline covers 10 to 50, so the colors change at 20. Measured over the
    // rectangle alone they would change at 25.
    assert_eq!(pixel_at(&pixels, 60, 14, 15), [0, 0, 255, 255]);
    assert_eq!(pixel_at(&pixels, 60, 22, 15), [255, 0, 0, 255]);
    assert_eq!(pixel_at(&pixels, 60, 45, 15), [255, 0, 0, 255]);
}

#[test]
fn unusable_sizes_are_errors_or_empty_layers() {
    for dimension in [0, -4] {
        for width in [true, false] {
            let mode = BitmapInterpolationMode::HighQuality;
            let error = resized(&mut io::empty(), dimension, width, mode).err();
            assert_eq!(error.map(|error| error.kind()), Some(io::ErrorKind::InvalidInput));
        }
    }
    let bitmap = Bitmap::new(PixelSize::new(4, 4), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(false, None);
    draw.clear(Colors::RED);
    // A layer without a size is empty. One over the device limit has no surface.
    assert!(!draw.create_layer(PixelSize::new(0, 8)).is_corrupted());
    assert!(draw
        .create_layer(PixelSize::new(20_000, 20_000))
        .is_corrupted());
    draw.dispose();
    let mut png = Vec::new();
    bitmap
        .save(
            &mut png,
            &ferroui_base::media::imaging::PngBitmapEncoderOptions::default().into(),
        )
        .unwrap();
    let half = resized(&mut &png[..], 2, true, BitmapInterpolationMode::HighQuality);
    assert_eq!(half.ok().map(|b| b.pixel_size()), Some(PixelSize::new(2, 2)));
    // A result over the device limit is a failed load.
    let oversized = resized(&mut &png[..], 20_000, true, BitmapInterpolationMode::HighQuality);
    assert!(oversized.is_err());
}

#[test]
fn compositing_layers_reject_values_that_cannot_be_composed() {
    use ferroui_base::media::imaging::BitmapBlendingMode;
    let bitmap = Bitmap::new(PixelSize::new(20, 20), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(false, None);
    draw.clear(Colors::WHITE);
    let bounds = Rect::new(0., 0., 20., 20.);
    let mode = BitmapBlendingMode::SourceOver;
    assert!(!draw.try_push_compositing_layer(mode, f64::NAN, bounds));
    assert!(!draw.try_push_compositing_layer(mode, 1.5, bounds));
    assert!(!draw.try_push_compositing_layer(mode, -0.5, bounds));
    assert!(!draw.try_push_compositing_layer(mode, 1., Rect::new(0., 0., f64::INFINITY, 20.)));
    assert!(draw.try_push_compositing_layer(mode, 0.5, bounds));
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::RED)),
        None,
        RoundedRect::from_rect(bounds),
        &Default::default(),
    );
    draw.pop_compositing_layer();
    draw.dispose();
    // A rejected layer was not pushed: the session ends balanced and keeps its pixels.
    let pixel = pixel_at(&bitmap.pixels(), 20, 10, 10);
    assert!(
        pixel[0] > 110 && pixel[0] < 145 && pixel[2] >= 254 && pixel[3] == 255,
        "{pixel:?}"
    );
}

#[test]
fn character_fallback_starts_from_the_system_font_for_unknown_families() {
    let manager = fonts::FontManager::default();
    // A Latin letter and a character that only the emoji font of the system has.
    for codepoint in [0x41, 0x1f600] {
        assert!(
            manager
                .try_match_character(
                    codepoint,
                    FontStyle::Normal,
                    FontWeight::Normal,
                    FontStretch::Normal,
                    Some("FerroUI deliberate missing font family"),
                    None,
                )
                .is_some(),
            "no fallback for U+{codepoint:04X}"
        );
    }
}

#[test]
fn embedded_families_select_the_nearest_face_and_report_its_weight() {
    use ferroui_base::media::fonts::OpenTypeTag;
    let manager = fonts::FontManager::default();
    let data = include_bytes!("../../FerroUI.Base/test_assets/fonts/InterVariable.ttf");
    let tag = OpenTypeTag::parse("wght");
    let mut family = String::new();
    // The heavy instance is loaded twice and is kept once.
    for weight in [900, 100, 900] {
        let face = manager
            .create_variable_typeface_from_stream(
                &mut &data[..],
                FontSimulations::None,
                &[(tag, f64::from(weight))],
            )
            .unwrap();
        assert_eq!(face.weight(), FontWeight(weight));
        family = face.family_name();
    }
    // Bold requests go to the heavier face and light requests to the lighter one. A
    // regular request has nothing in 400 to 500 and takes the lighter face.
    for (requested, nearest) in [(700, 900), (800, 900), (200, 100), (400, 100)] {
        let face = manager
            .try_create_glyph_typeface(
                &family,
                FontStyle::Normal,
                FontWeight(requested),
                FontStretch::Normal,
            )
            .unwrap();
        assert_eq!(face.weight(), FontWeight(nearest), "weight {requested}");
    }
}
