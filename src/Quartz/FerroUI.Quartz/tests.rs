use super::*;
use ferroui_base::{
    media::{
        immutable::ImmutableSolidColorBrush, text_formatting::TextShaperOptions, Colors,
        FontSimulations, FontStretch, FontStyle, FontWeight,
    },
    utilities::ReadOnlyMemory,
};
use ferroui_base::{Matrix, RoundedRect};
use kurbo::Shape;
fn face() -> Rc<GlyphTypeface> {
    let manager = fonts::FontManager::default();
    let platform = manager
        .try_create_glyph_typeface(
            "Helvetica",
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
        )
        .unwrap();
    GlyphTypeface::new(platform, FontSimulations::None).unwrap()
}
#[test]
fn core_text_shapes_utf16_subranges_and_rtl() {
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
fn system_geometry_boolean_operations_preserve_holes() {
    let a = Geometry::rectangle(Rect::new(0., 0., 20., 20.));
    let b = Geometry::rectangle(Rect::new(5., 5., 10., 10.));
    let difference = Geometry::combine(&*a, &*b, GeometryCombineMode::Exclude);
    assert!(difference.fill_contains(Point::new(2., 2.)));
    assert!(!difference.fill_contains(Point::new(10., 10.)));
    let intersection = Geometry::combine(&*a, &*b, GeometryCombineMode::Intersect);
    assert!(!intersection.fill_contains(Point::new(2., 2.)));
    assert!(intersection.fill_contains(Point::new(10., 10.)));
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
fn imageio_roundtrips_png_and_dpi_rendering_scales() {
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
fn core_text_glyphs_draw_visible_pixels() {
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
fn singular_transform_does_not_poison_the_next_draw() {
    let bitmap = Bitmap::new(PixelSize::new(20, 20), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(false, None);
    draw.clear(Colors::WHITE);
    draw.set_transform(Matrix::new(0., 0., 0., 0., 0., 0.));
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::BLUE)),
        None,
        RoundedRect::from_rect(Rect::new(0., 0., 20., 20.)),
        &Default::default(),
    );
    draw.set_transform(Matrix::IDENTITY);
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::RED)),
        None,
        RoundedRect::from_rect(Rect::new(0., 0., 20., 20.)),
        &Default::default(),
    );
    draw.dispose();
    assert!(bitmap
        .pixels()
        .chunks_exact(4)
        .all(|p| p == [0, 0, 255, 255]));
}

#[test]
fn hidden_system_family_resolves_bold_and_missing_families_stay_missing() {
    let manager = fonts::FontManager::default();
    let family = manager.get_default_font_family_name();
    for weight in [FontWeight::Normal, FontWeight::Bold] {
        let platform = manager
            .try_create_glyph_typeface(&family, FontStyle::Normal, weight, FontStretch::Normal)
            .expect("system font must have a bold face");
        let face = GlyphTypeface::new(platform, FontSimulations::None).unwrap();
        let shaped = fonts::TextShaper.shape_text(
            &ReadOnlyMemory::from_str("FerroUI"),
            &TextShaperOptions::new(face),
        );
        assert!(shaped.glyph_infos().iter().all(|g| g.glyph_index != 0));
    }
    assert!(manager
        .try_create_glyph_typeface(
            "MissingFerroPrototypeFont123",
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal
        )
        .is_none());
}

#[test]
fn color_glyphs_have_ink_bounds_without_outlines() {
    let manager = fonts::FontManager::default();
    let platform = manager
        .try_match_character(
            '😀' as i32,
            FontStyle::Normal,
            FontWeight::Normal,
            FontStretch::Normal,
            None,
            None,
        )
        .unwrap();
    let face = GlyphTypeface::new(platform, FontSimulations::None).unwrap();
    let text = ReadOnlyMemory::from_str("😀");
    let options = TextShaperOptions::with_all(face.clone(), 24., 0, None, 0., 0., None);
    let shaped = fonts::TextShaper.shape_text(&text, &options);
    let glyphs = shaped.glyph_infos().to_vec();
    let run = GlyphRun::new(&face, 24., &glyphs, Point::new(0., 30.));
    assert!(run.bounds.width > 0.0 && run.bounds.height > 0.0);
    assert!(glyphs.iter().all(|g| g.glyph_index != 0));
    let bitmap = Bitmap::new(PixelSize::new(40, 40), Vector::new(96., 96.));
    let mut draw = bitmap.drawing(false, None);
    draw.draw_glyph_run(Some(&ImmutableSolidColorBrush::new(Colors::BLACK)), &run);
    draw.dispose();
    let pixels = bitmap.pixels();
    assert!(
        pixels
            .chunks_exact(4)
            .filter(|p| p[3] > 0 && p[0] != p[2])
            .count()
            > 20
    );
    // The historical CPU reference has narrower brush support; this checks
    // the native GPU path's intrinsic color and brush-alpha composition.
    #[cfg(feature = "metal")]
    {
        use ferroui_base::media::{
            immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush},
            GradientSpreadMethod,
        };
        use ferroui_base::{RelativePoint, RelativeUnit};
        for opacity in [1., 0.5] {
            let brush = ImmutableLinearGradientBrush::new(
                &[
                    ImmutableGradientStop::new(0., Colors::RED),
                    ImmutableGradientStop::new(1., Colors::BLUE),
                ],
                opacity,
                None,
                None,
                GradientSpreadMethod::Pad,
                Some(RelativePoint::new(0., 0., RelativeUnit::Relative)),
                Some(RelativePoint::new(1., 0., RelativeUnit::Relative)),
                None,
            );
            let gradient = Bitmap::new(PixelSize::new(40, 40), Vector::new(96., 96.));
            let mut draw = gradient.drawing(false, None);
            draw.draw_glyph_run(Some(&brush), &run);
            draw.dispose();
            let result = gradient.pixels();
            assert!(
                result
                    .chunks_exact(4)
                    .filter(|p| p[3] > 0 && p[0] != p[2])
                    .count()
                    > 20,
                "A gradient brush must preserve intrinsic color glyphs"
            );
            for (original, pixel) in pixels.iter().zip(&result) {
                let expected = f64::from(*original) * opacity;
                assert!(
                    (f64::from(*pixel) - expected).abs() <= 2.,
                    "opacity={opacity}: {original} -> {pixel}, expected {expected}"
                );
            }
        }
        let fading = ImmutableLinearGradientBrush::new(
            &[
                ImmutableGradientStop::new(0., ferroui_base::media::Color::from_argb(0, 255, 0, 0)),
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
        let faded = Bitmap::new(PixelSize::new(40, 40), Vector::new(96., 96.));
        let mut draw = faded.drawing(false, None);
        draw.draw_glyph_run(Some(&fading), &run);
        draw.dispose();
        for (i, (source, result)) in pixels
            .chunks_exact(4)
            .zip(faded.pixels().chunks_exact(4))
            .enumerate()
        {
            let factor = (((i % 40) as f64 + 0.5 - run.bounds.x) / run.bounds.width).clamp(0., 1.);
            assert!(
                (f64::from(result[3]) - f64::from(source[3]) * factor).abs() <= 3.,
                "gradient alpha at {i}: {source:?} -> {result:?}"
            );
        }
    }
}

#[cfg(feature = "metal")]
#[test]
fn metal_texture_copy_resize_and_composition() {
    let source = Bitmap::new(PixelSize::new(8, 8), Vector::new(96., 96.));
    let mut draw = source.drawing(false, None);
    draw.clear(Colors::TRANSPARENT);
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::RED)),
        None,
        RoundedRect::from_rect(Rect::new(0., 0., 8., 4.)),
        &Default::default(),
    );
    draw.dispose();
    let resized = Bitmap::resized(
        &source,
        PixelSize::new(16, 16),
        ferroui_base::media::imaging::BitmapInterpolationMode::Unspecified,
    );
    let snapshot = resized.create_shared_snapshot();
    let target = Bitmap::new(PixelSize::new(16, 16), Vector::new(96., 96.));
    let mut draw = target.drawing(false, None);
    draw.draw_bitmap(
        &*snapshot,
        1.,
        Rect::new(0., 0., 16., 16.),
        Rect::new(0., 0., 16., 16.),
    );
    draw.dispose();
    let pixels = target.pixels();
    assert_eq!(&pixels[(2 * 16 + 2) * 4..][..4], &[0, 0, 255, 255]);
    assert_eq!(&pixels[(13 * 16 + 2) * 4..][..4], &[0, 0, 0, 0]);
    assert_eq!(metal::statistics()[6], 0);
}

#[cfg(feature = "metal")]
#[test]
fn metal_linear_gradient_and_shadow_dispatch_outside_path() {
    let bitmap = Bitmap::new(PixelSize::new(40, 40), Vector::new(96., 96.));
    let path = Geometry::rectangle(Rect::new(0., 0., 40., 10.));
    let native = geometry::native(path.fill.as_ref().unwrap());
    bitmap.context(|c| unsafe {
        ffi::q_save(c);
        ffi::q_clip(c, native.0, 0);
        ffi::q_gradient(
            c,
            [1., 0., 0., 1., 0., 0., 1., 1.].as_ptr(),
            [0., 1.].as_ptr(),
            2,
            0.,
            0.,
            40.,
            0.,
            0.,
            0,
        );
        ffi::q_restore(c);
    });
    let path = Geometry::rectangle(Rect::new(5., 15., 5., 5.));
    let native = geometry::native(path.fill.as_ref().unwrap());
    bitmap.context(|c| unsafe {
        ffi::q_shadow(c, 10., 0., 2., 0., 0., 0., 1.);
        ffi::q_fill(c, native.0, 0, 1., 0., 0., 1.);
    });
    let p = bitmap.pixels();
    assert!(p[(5 * 40 + 2) * 4 + 2] > 230 && p[(5 * 40 + 37) * 4] > 230);
    assert_eq!(p[(12 * 40 + 2) * 4 + 3], 0);
    assert!(p[(17 * 40 + 17) * 4 + 3] > 230); // shifted shadow is outside the fill's bounding box
}

#[test]
fn textbox_border_brush_reflection_puts_the_highlight_at_the_bottom() {
    use ferroui_base::{
        media::{
            immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutableTransform},
            GradientSpreadMethod,
        },
        RelativePoint, RelativeUnit,
    };
    let brush = ImmutableLinearGradientBrush::new(
        &[
            ImmutableGradientStop::new(0.5, Colors::WHITE),
            ImmutableGradientStop::new(1., Colors::BLACK),
        ],
        1.,
        Some(Rc::new(ImmutableTransform::new(Matrix::create_scale(
            1., -1.,
        )))),
        Some(RelativePoint::new(0., 0.5, RelativeUnit::Relative)),
        GradientSpreadMethod::Pad,
        Some(RelativePoint::new(0., 0., RelativeUnit::Absolute)),
        Some(RelativePoint::new(0., 2., RelativeUnit::Absolute)),
        None,
    );
    // Apply the same absolute paint transform as the WinUI TextBox theme under
    // a translated, high-DPI visual. The clip must stay at its original location.
    let bitmap = Bitmap::new(PixelSize::new(60, 60), Vector::new(192., 192.));
    let mut draw = bitmap.drawing(true, None);
    draw.set_transform(Matrix::create_translation(3., 4.));
    draw.draw_rectangle(
        Some(&brush),
        None,
        RoundedRect::from_rect(Rect::new(0., 0., 20., 20.)),
        &Default::default(),
    );
    draw.dispose();
    let pixels = bitmap.pixels();
    assert_eq!(pixels[(10 * 60 + 20) * 4], 0); // top is dark
    assert!(pixels[(47 * 60 + 20) * 4] > 200); // reflected two-unit highlight at bottom
    assert_eq!(pixels[(3 * 60 + 20) * 4 + 3], 0); // clip did not move with the brush
}
#[test]
fn variable_fonts_simulations_family_faces_and_underline_ink_are_available() {
    use ferroui_base::media::fonts::OpenTypeTag;
    let manager = fonts::FontManager::default();
    let data = include_bytes!("../../Skia/FerroUI.Skia/test_assets/assets/InterVariable.ttf");
    let axis = OpenTypeTag::parse("wght");
    let light = manager
        .create_variable_typeface_from_stream(
            &mut &data[..],
            FontSimulations::None,
            &[(axis, 100.)],
        )
        .unwrap();
    let heavy = manager
        .create_variable_typeface_from_stream(
            &mut &data[..],
            FontSimulations::None,
            &[(axis, 900.)],
        )
        .unwrap();
    assert!(manager
        .create_variable_typeface_from_stream(
            &mut &data[..],
            FontSimulations::None,
            &[(axis, 10000.)]
        )
        .is_none());
    assert!(manager
        .create_variable_typeface_from_stream(
            &mut &data[..],
            FontSimulations::None,
            &[(OpenTypeTag::parse("fake"), 1.)]
        )
        .is_none());
    let glyph = |face: Rc<dyn ferroui_base::media::IPlatformTypeface>| {
        let face = GlyphTypeface::new(face, FontSimulations::None).unwrap();
        let id = face.character_to_glyph_map().get_glyph('g' as i32);
        GlyphRun::new(
            &face,
            40.,
            &[GlyphInfo::with_offset(id, 0, 30., Vector::new(0., 0.))],
            Point::new(8., 44.),
        )
    };
    let light = glyph(light);
    let heavy = glyph(heavy);
    let area = |g: &GlyphRun| g.outlines.area().abs();
    assert!(area(&heavy) > area(&light) * 1.3);
    let regular = include_bytes!("../../Skia/FerroUI.Skia/test_assets/assets/Inter-Regular.ttf");
    let normal = glyph(
        manager
            .try_create_glyph_typeface_from_stream(&mut &regular[..], FontSimulations::None)
            .unwrap(),
    );
    let simulated = glyph(
        manager
            .try_create_glyph_typeface_from_stream(
                &mut &regular[..],
                FontSimulations::Bold | FontSimulations::Oblique,
            )
            .unwrap(),
    );
    assert!(area(&simulated) > area(&normal));
    assert_ne!(simulated.bounds, normal.bounds);
    let intersections = normal.get_intersections(2., 4.);
    assert!(!intersections.is_empty());
    assert_eq!(intersections.len() % 2, 0);
    assert!(intersections.chunks_exact(2).all(|p| p[1] > p[0]));
    assert!(normal.get_intersections(100., 110.).is_empty());
    let faces = manager.try_get_family_typefaces("Helvetica").unwrap();
    assert!(faces.iter().any(|f| f.weight().0 >= 600));
    assert!(faces.iter().any(|f| f.style() != FontStyle::Normal));
    assert!(manager
        .try_get_family_typefaces("Ferro Missing Family 12345")
        .is_none());
}

#[test]
fn contextual_arabic_run_shaping_preserves_joining_and_local_clusters() {
    let manager = fonts::FontManager::default();
    let data =
        include_bytes!("../../Skia/FerroUI.Skia/test_assets/assets/NotoSansArabic-Regular.ttf");
    let platform = manager
        .try_create_glyph_typeface_from_stream(&mut &data[..], FontSimulations::None)
        .unwrap();
    let face = GlyphTypeface::new(platform, FontSimulations::None).unwrap();
    let options = TextShaperOptions::with_all(face, 30., 1, None, 0., 0., None);
    let text = ReadOnlyMemory::from_str("ببب");
    let full = fonts::TextShaper.shape_text(&text, &options);
    let middle = fonts::TextShaper.shape_text(&text.slice(1, 1), &options);
    assert_eq!(middle.length(), 1);
    let expected = full
        .glyph_infos()
        .iter()
        .find(|g| g.glyph_cluster == 1)
        .unwrap()
        .glyph_index;
    assert_eq!(middle.glyph_infos()[0].glyph_index, expected);
    assert_eq!(middle.glyph_infos()[0].glyph_cluster, 0);
}

#[cfg(feature = "metal")]
#[test]
fn native_hinted_glyph_masks_are_dpi_aware_tinted_on_gpu_and_cached() {
    use ferroui_base::media::{TextHintingMode, TextOptions};
    const FLAG: &str = "FERRO_QUARTZ_ISOLATED_GLYPH_TEST";
    if std::env::var_os(FLAG).is_none() {
        assert!(std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::native_hinted_glyph_masks_are_dpi_aware_tinted_on_gpu_and_cached",
                "--nocapture"
            ])
            .env(FLAG, "1")
            .status()
            .unwrap()
            .success());
        return;
    }
    let face = face();
    let id = face.character_to_glyph_map().get_glyph('F' as i32);
    let run = GlyphRun::new(
        &face,
        20.,
        &[GlyphInfo::with_offset(id, 0, 20., Vector::new(0., 0.))],
        Point::new(4., 24.),
    );
    let b = Bitmap::new(PixelSize::new(80, 80), Vector::new(192., 192.));
    let draw = |color: ferroui_base::media::Color| {
        let mut d = b.drawing(true, None);
        d.clear(Colors::TRANSPARENT);
        d.push_text_options(TextOptions {
            text_hinting_mode: TextHintingMode::Strong,
            ..Default::default()
        });
        d.draw_glyph_run(Some(&ImmutableSolidColorBrush::new(color)), &run);
        d.pop_text_options();
        d.dispose();
    };
    draw(Colors::RED);
    let red = b.pixels();
    let uploads = crate::metal_statistics()[5];
    let mut min_y = 80;
    let mut max_y = 0;
    let mut ink = 0;
    for (i, p) in red.chunks_exact(4).enumerate() {
        if p[3] > 0 {
            min_y = min_y.min(i / 80);
            max_y = max_y.max(i / 80);
            ink += 1;
            assert_eq!(p[0], 0);
            assert_eq!(p[1], 0);
            assert_eq!(p[2], p[3]);
        }
    }
    assert!(ink > 100);
    assert!(min_y >= 10 && min_y < 25, "min={min_y}");
    assert!(max_y >= 40 && max_y <= 50, "max={max_y}");
    draw(Colors::BLUE);
    let blue = b.pixels();
    assert_eq!(
        crate::metal_statistics()[5],
        uploads,
        "Changing tint must reuse the glyph mask"
    );
    for (r, b) in red.chunks_exact(4).zip(blue.chunks_exact(4)) {
        assert_eq!(r[2], b[0]);
        assert_eq!(r[3], b[3]);
        assert_eq!(b[2], 0);
    }
}

#[test]
fn system_font_descriptors_preserve_all_standard_weights_in_the_ui_family() {
    let manager = fonts::FontManager::default();
    let family = manager.get_default_font_family_name();
    let mut previous = 0.;
    for weight in (100..=900).step_by(100) {
        let platform = manager
            .try_create_glyph_typeface(
                &family,
                FontStyle::Normal,
                FontWeight(weight),
                FontStretch::Normal,
            )
            .unwrap();
        assert_eq!(platform.family_name(), family);
        assert_eq!(platform.font_simulations(), FontSimulations::None);
        let face = GlyphTypeface::new(platform, FontSimulations::None).unwrap();
        let id = face.character_to_glyph_map().get_glyph('H' as i32);
        let run = GlyphRun::new(
            &face,
            40.,
            &[GlyphInfo::with_offset(id, 0, 40., Vector::new(0., 0.))],
            Point::new(0., 40.),
        );
        let area = run.outlines.area().abs();
        assert!(
            area > previous + 1.,
            "Weight {weight} did not increase ink: {previous} -> {area}"
        );
        previous = area;
    }
    let styles = manager.try_get_family_typefaces(&family).unwrap();
    for weight in (100..=900).step_by(100) {
        assert!(styles.iter().any(|f| f.weight().0 == weight));
    }
}
