use super::*;
use ferroui_base::{
    media::{
        immutable::ImmutableSolidColorBrush, Color, ColorMatrix, ColorMatrixAlphaMode, Colors,
        ConvolveMatrix, ImmutableBlurEffect, ImmutableDropShadowEffect, RenderOptions,
    },
    RoundedRect,
};
use std::any::Any;
fn bitmap() -> Bitmap {
    Bitmap::new(PixelSize::new(64, 64), Vector::new(96., 96.))
}
fn rectangle(draw: &mut dyn IDrawingContextImpl, color: Color, rect: Rect) {
    draw.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(color)),
        None,
        RoundedRect::from_rect(rect),
        &Default::default(),
    );
}
fn pixel(data: &[u8], x: usize, y: usize) -> [u8; 4] {
    data[(y * 64 + x) * 4..][..4].try_into().unwrap()
}
#[test]
fn gaussian_blur_spreads_beyond_content_bounds_and_preserves_outer_clip() {
    let b = bitmap();
    let mut d = b.drawing(false, None);
    d.push_clip(Rect::new(0., 0., 38., 64.));
    d.as_drawing_context_impl_with_effects()
        .unwrap()
        .push_effect(
            Some(Rect::new(24., 24., 16., 16.)),
            &ImmutableBlurEffect::new(10.),
        );
    rectangle(&mut d, Colors::RED, Rect::new(0., 0., 64., 64.));
    d.as_drawing_context_impl_with_effects()
        .unwrap()
        .pop_effect();
    d.pop_clip();
    d.dispose();
    let p = b.pixels();
    let outside = pixel(&p, 21, 32);
    let center = pixel(&p, 30, 32);
    assert!(outside[3] > 0 && outside[3] < 100, "{outside:?}");
    assert_eq!(outside[2], outside[3]);
    assert!(center[3] > 200);
    assert_eq!(pixel(&p, 45, 32), [0; 4]);
    assert_eq!(pixel(&p, 1, 1), [0; 4]);
}
#[test]
fn color_matrices_match_reference_and_do_not_color_outside_the_layer() {
    for mode in [
        ColorMatrixAlphaMode::Premultiplied,
        ColorMatrixAlphaMode::Straight,
    ] {
        let b = bitmap();
        let mut d = b.drawing(false, None);
        let matrix = ColorMatrix::scale_and_offset([0.5, 1.2, 0.2, 0.8], [0.1, 0., 0.2, 0.05]);
        d.as_drawing_context_impl_with_layer_effects()
            .unwrap()
            .push_color_matrix_layer(Rect::new(4., 4., 16., 16.), &matrix, mode);
        let c = Color::from_argb(200, 80, 120, 160);
        rectangle(&mut d, c, Rect::new(4., 4., 16., 16.));
        d.as_drawing_context_impl_with_layer_effects()
            .unwrap()
            .pop_color_matrix_layer();
        d.dispose();
        let p = b.pixels();
        let out = pixel(&p, 10, 10);
        let mut input = [63u8, 94, 125, 200];
        matrix.apply_to_rgba8(&mut input, mode);
        input.swap(0, 2);
        for i in 0..4 {
            assert!(
                (i32::from(out[i]) - i32::from(input[i])).abs() <= 2,
                "{mode:?} {out:?} {input:?}"
            );
        }
        assert_eq!(pixel(&p, 0, 0), [0; 4]);
    }
}
#[test]
fn multiply_factors_brighten_on_gpu_without_changing_alpha() {
    let b = bitmap();
    let mut d = b.drawing(false, None);
    let factors = MultiplyLayerFactors {
        size: PixelSize::new(1, 1),
        rgba: Arc::new(vec![64, 128, 255, 255]),
        scale: 2.,
    };
    d.as_drawing_context_impl_with_layer_effects()
        .unwrap()
        .push_multiply_layer(Rect::new(4., 4., 20., 20.), &factors);
    rectangle(
        &mut d,
        Color::from_argb(200, 128, 128, 128),
        Rect::new(0., 0., 64., 64.),
    );
    d.as_drawing_context_impl_with_layer_effects()
        .unwrap()
        .pop_multiply_layer();
    d.dispose();
    let p = b.pixels();
    let out = pixel(&p, 10, 10);
    assert!((i32::from(out[2]) - 50).abs() <= 2);
    assert!((i32::from(out[1]) - 100).abs() <= 2);
    assert_eq!(out[0], 200);
    assert_eq!(out[3], 200);
    assert_eq!(pixel(&p, 1, 1), [0; 4]);
}
#[test]
fn convolution_matches_the_shared_pixel_reference() {
    for hard in [false, true] {
        for preserve in [false, true] {
            let b = bitmap();
            let mut d = b.drawing(false, None);
            let mut kernel = ConvolveMatrix::new(3, 1, &[0.25, 0.5, 0.25]).unwrap();
            kernel.clamp_edges = hard;
            kernel.preserve_alpha = preserve;
            kernel.bias = 0.05;
            d.as_drawing_context_impl_with_layer_effects()
                .unwrap()
                .push_convolve_matrix_layer(Rect::new(4., 4., 8., 8.), &kernel);
            rectangle(
                &mut d,
                Color::from_argb(128, 100, 200, 50),
                Rect::new(4., 4., 8., 8.),
            );
            d.as_drawing_context_impl_with_layer_effects()
                .unwrap()
                .pop_convolve_matrix_layer();
            d.dispose();
            let p = b.pixels();
            let mut expected: Vec<u8> = std::iter::repeat_n([50, 100, 25, 128], 64)
                .flatten()
                .collect();
            kernel.apply_to_premultiplied_rgba8(&mut expected, 8, 8);
            for y in 0..8 {
                for x in 0..8 {
                    let mut e: [u8; 4] = expected[(y * 8 + x) * 4..][..4].try_into().unwrap();
                    e.swap(0, 2);
                    let o = pixel(&p, x + 4, y + 4);
                    for i in 0..4 {
                        assert!(
                            (i32::from(o[i]) - i32::from(e[i])).abs() <= 2,
                            "hard={hard} preserve={preserve} at {x},{y}: {o:?} {e:?}"
                        );
                    }
                }
            }
            assert_eq!(pixel(&p, 0, 0), [0; 4]);
        }
    }
}

#[test]
fn a_drop_shadow_keeps_the_content_and_offsets_its_alpha_on_gpu() {
    let b = bitmap();
    let mut d = b.drawing(false, None);
    d.as_drawing_context_impl_with_effects()
        .unwrap()
        .push_effect(
            None,
            &ImmutableDropShadowEffect::new(12., 4., 0., Colors::BLACK, 0.5),
        );
    rectangle(&mut d, Colors::RED, Rect::new(4., 4., 8., 8.));
    d.as_drawing_context_impl_with_effects()
        .unwrap()
        .pop_effect();
    d.dispose();
    let p = b.pixels();
    assert_eq!(pixel(&p, 7, 7), [0, 0, 255, 255]);
    let shadow = pixel(&p, 19, 11);
    assert!(shadow[3] >= 127 && shadow[3] <= 129);
    assert_eq!(&shadow[..3], &[0, 0, 0]);
    assert_eq!(pixel(&p, 40, 40), [0; 4]);
}

#[test]
fn conic_gradients_start_above_the_center_and_rotate_clockwise() {
    use ferroui_base::media::{
        immutable::{ImmutableConicGradientBrush, ImmutableGradientStop},
        GradientSpreadMethod,
    };
    let stops = [
        ImmutableGradientStop::new(0., Colors::RED),
        ImmutableGradientStop::new(0.24, Colors::RED),
        ImmutableGradientStop::new(0.25, Colors::GREEN),
        ImmutableGradientStop::new(0.49, Colors::GREEN),
        ImmutableGradientStop::new(0.5, Colors::BLUE),
        ImmutableGradientStop::new(1., Colors::BLUE),
    ];
    for angle in [0., 90.] {
        let b = bitmap();
        let mut d = b.drawing(false, None);
        let brush = ImmutableConicGradientBrush::new(
            &stops,
            1.,
            None,
            None,
            GradientSpreadMethod::Pad,
            None,
            angle,
            None,
        );
        d.draw_rectangle(
            Some(&brush),
            None,
            RoundedRect::from_rect(Rect::new(0., 0., 64., 64.)),
            &Default::default(),
        );
        d.dispose();
        let p = b.pixels();
        if angle == 0. {
            assert!(pixel(&p, 32, 8)[2] > 250);
            assert!(pixel(&p, 55, 32)[1] > 100);
        } else {
            assert!(pixel(&p, 55, 32)[2] > 250);
            assert!(pixel(&p, 32, 8)[0] > 250);
        }
        assert!(pixel(&p, 8, 31)[0] > 250);
    }
}
#[test]
fn linear_gradients_repeat_and_reflect_in_brush_space() {
    use ferroui_base::{
        media::{
            immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush},
            GradientSpreadMethod,
        },
        RelativePoint, RelativeUnit,
    };
    let stops = [
        ImmutableGradientStop::new(0., Colors::RED),
        ImmutableGradientStop::new(1., Colors::BLUE),
    ];
    for spread in [GradientSpreadMethod::Repeat, GradientSpreadMethod::Reflect] {
        let b = bitmap();
        let mut d = b.drawing(false, None);
        let brush = ImmutableLinearGradientBrush::new(
            &stops,
            1.,
            None,
            None,
            spread,
            Some(RelativePoint::new(0., 0., RelativeUnit::Absolute)),
            Some(RelativePoint::new(16., 0., RelativeUnit::Absolute)),
            None,
        );
        d.draw_rectangle(
            Some(&brush),
            None,
            RoundedRect::from_rect(Rect::new(0., 0., 64., 64.)),
            &Default::default(),
        );
        d.dispose();
        let p = b.pixels();
        let a = pixel(&p, 4, 10);
        let b = pixel(
            &p,
            if spread == GradientSpreadMethod::Repeat {
                20
            } else {
                27
            },
            10,
        );
        for i in 0..4 {
            assert!(
                (i32::from(a[i]) - i32::from(b[i])).abs() <= 1,
                "{spread:?} {a:?} {b:?}"
            );
        }
    }
}
#[test]
fn radial_gradients_support_ellipses_and_a_distinct_focal_point() {
    use ferroui_base::{
        media::{
            immutable::{ImmutableGradientStop, ImmutableRadialGradientBrush},
            GradientSpreadMethod,
        },
        RelativePoint, RelativeScalar, RelativeUnit,
    };
    let stops = [
        ImmutableGradientStop::new(0., Colors::RED),
        ImmutableGradientStop::new(1., Colors::BLUE),
    ];
    for focal in [false, true] {
        let b = bitmap();
        let mut d = b.drawing(false, None);
        let brush = ImmutableRadialGradientBrush::new(
            &stops,
            1.,
            None,
            None,
            GradientSpreadMethod::Pad,
            Some(RelativePoint::new(32., 32., RelativeUnit::Absolute)),
            Some(RelativePoint::new(
                if focal { 20. } else { 32. },
                32.,
                RelativeUnit::Absolute,
            )),
            Some(RelativeScalar::new(24., RelativeUnit::Absolute)),
            Some(RelativeScalar::new(
                if focal { 24. } else { 8. },
                RelativeUnit::Absolute,
            )),
            None,
        );
        d.draw_rectangle(
            Some(&brush),
            None,
            RoundedRect::from_rect(Rect::new(0., 0., 64., 64.)),
            &Default::default(),
        );
        d.dispose();
        let p = b.pixels();
        if focal {
            assert!(pixel(&p, 20, 32)[2] > 240);
            assert!(pixel(&p, 32, 32)[0] > 70);
        } else {
            assert_eq!(pixel(&p, 32, 44), [255, 0, 0, 255]);
            assert!(pixel(&p, 44, 32)[2] > 100);
        }
    }
}

#[test]
fn image_brush_tiles_flip_both_axes_and_apply_brush_transforms() {
    use ferroui_base::{
        media::{
            imaging::BitmapInterpolationMode,
            immutable::{ImmutableImageBrush, ImmutableTransform},
            AlignmentX, AlignmentY, Stretch, TileMode,
        },
        Matrix, RelativePoint, RelativeRect, RelativeUnit,
    };
    let source = Bitmap::from_pixels(
        PixelFormat::RGBA8888,
        AlphaFormat::Opaque,
        &[
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ],
        PixelSize::new(2, 2),
        Vector::new(96., 96.),
        8,
    );
    let source = Rc::new(ferroui_base::media::imaging::Bitmap::from_impl(Arc::new(
        source,
    )));
    for mode in [
        TileMode::None,
        TileMode::Tile,
        TileMode::FlipX,
        TileMode::FlipY,
        TileMode::FlipXY,
    ] {
        for transformed in [false, true] {
            let brush = ImmutableImageBrush::new(
                Some(source.clone()),
                AlignmentX::Left,
                AlignmentY::Top,
                Some(RelativeRect::new(0., 0., 8., 8., RelativeUnit::Absolute)),
                1.,
                if transformed {
                    Some(Rc::new(ImmutableTransform::new(
                        Matrix::create_translation(8., 4.),
                    )))
                } else {
                    None
                },
                RelativePoint::TOP_LEFT,
                None,
                Stretch::Fill,
                mode,
                None,
            );
            let b = bitmap();
            let mut d = b.drawing(false, None);
            d.push_render_options(RenderOptions {
                bitmap_interpolation_mode: BitmapInterpolationMode::None,
                ..Default::default()
            });
            d.draw_rectangle(
                Some(&brush),
                None,
                RoundedRect::from_rect(Rect::new(0., 0., 32., 32.)),
                &Default::default(),
            );
            d.pop_render_options();
            d.dispose();
            let p = b.pixels();
            let (x, y) = if transformed { (9, 5) } else { (1, 1) };
            assert_eq!(
                pixel(&p, x, y),
                [0, 0, 255, 255],
                "{mode:?} transformed={transformed}"
            );
            if mode == TileMode::None {
                assert_eq!(pixel(&p, x + 8, y), [0; 4]);
            } else {
                assert_eq!(
                    pixel(&p, x + 8, y),
                    if mode == TileMode::FlipX || mode == TileMode::FlipXY {
                        [0, 255, 0, 255]
                    } else {
                        [0, 0, 255, 255]
                    }
                );
                assert_eq!(
                    pixel(&p, x, y + 8),
                    if mode == TileMode::FlipY || mode == TileMode::FlipXY {
                        [255, 0, 0, 255]
                    } else {
                        [0, 0, 255, 255]
                    }
                );
            }
            assert_eq!(pixel(&p, 35, 15), [0; 4]); // paint transform never moves the geometry clip
        }
    }
}

#[test]
fn render_options_merge_restore_and_control_coverage_and_bitmap_sampling() {
    use ferroui_base::media::{
        imaging::{BitmapBlendingMode, BitmapInterpolationMode},
        EdgeMode,
    };
    let source = Bitmap::from_pixels(
        PixelFormat::RGBA8888,
        AlphaFormat::Opaque,
        &[255, 0, 0, 255, 0, 0, 255, 255],
        PixelSize::new(2, 1),
        Vector::new(96., 96.),
        8,
    );
    let b = bitmap();
    let mut d = b.drawing(false, None);
    d.push_render_options(RenderOptions {
        bitmap_interpolation_mode: BitmapInterpolationMode::None,
        edge_mode: EdgeMode::Aliased,
        ..Default::default()
    });
    d.push_render_options(RenderOptions {
        bitmap_blending_mode: BitmapBlendingMode::Source,
        ..Default::default()
    });
    rectangle(&mut d, Colors::RED, Rect::new(0.4, 16., 8., 8.));
    d.draw_bitmap(
        &source,
        1.,
        Rect::new(0., 0., 2., 1.),
        Rect::new(0., 0., 16., 8.),
    );
    d.pop_render_options();
    d.pop_render_options();
    rectangle(&mut d, Colors::RED, Rect::new(0.4, 32., 8., 8.));
    d.draw_bitmap(
        &source,
        1.,
        Rect::new(0., 0., 2., 1.),
        Rect::new(0., 40., 16., 8.),
    );
    d.dispose();
    let p = b.pixels();
    assert_eq!(pixel(&p, 7, 3), [0, 0, 255, 255]);
    assert_eq!(pixel(&p, 8, 3), [255, 0, 0, 255]);
    let edge = pixel(&p, 0, 18);
    assert_eq!(edge[3], 255);
    let smooth = pixel(&p, 0, 34);
    assert!(smooth[3] > 0 && smooth[3] < 255);
    let mixed = pixel(&p, 7, 43);
    assert!(mixed[0] > 50 && mixed[2] > 50, "{mixed:?}");
}

#[test]
fn box_shadows_apply_spread_inset_order_and_leave_transparent_casters_empty() {
    use ferroui_base::media::{BoxShadow, BoxShadows};
    let rect = RoundedRect::from_rect(Rect::new(16., 16., 16., 16.));
    let b = bitmap();
    let mut d = b.drawing(false, None);
    d.draw_rectangle(
        None,
        None,
        rect,
        &BoxShadows::new(BoxShadow {
            spread: 2.,
            color: Colors::BLACK,
            ..Default::default()
        }),
    );
    d.dispose();
    let p = b.pixels();
    assert_eq!(pixel(&p, 14, 24), [0, 0, 0, 255]);
    assert_eq!(pixel(&p, 24, 24), [0; 4]);
    assert_eq!(pixel(&p, 12, 24), [0; 4]);
    let b = bitmap();
    let mut d = b.drawing(false, None);
    d.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(Colors::WHITE)),
        None,
        rect,
        &BoxShadows::new(BoxShadow {
            offset_x: 4.,
            is_inset: true,
            color: Colors::BLACK,
            ..Default::default()
        }),
    );
    d.dispose();
    let p = b.pixels();
    assert_eq!(pixel(&p, 17, 24), [0, 0, 0, 255]);
    assert_eq!(pixel(&p, 25, 24), [255; 4]);
    assert_eq!(pixel(&p, 14, 24), [0; 4]);
}

#[test]
fn pixel_operations_fold_sources_on_gpu_and_match_the_shared_reference() {
    use ferroui_base::{
        platform::PixelOperation, rendering::composition::effects::BlendEffectMode,
    };
    let mut operations = vec![
        PixelOperation::Arithmetic {
            k: [0.3, 0.4, 0.7, 0.02],
        },
        PixelOperation::MaskInvert,
    ];
    for mode in [
        BlendEffectMode::Dissolve,
        BlendEffectMode::LinearBurn,
        BlendEffectMode::DarkerColor,
        BlendEffectMode::LighterColor,
        BlendEffectMode::VividLight,
        BlendEffectMode::LinearLight,
        BlendEffectMode::PinLight,
        BlendEffectMode::HardMix,
        BlendEffectMode::Subtract,
        BlendEffectMode::Division,
    ] {
        operations.push(PixelOperation::Blend {
            mode,
            noise_grid: Rect::new(0., 0., 64., 64.),
            noise_size: PixelSize::new(64, 64),
        });
    }
    let colors = [[20, 40, 60, 128], [90, 50, 25, 192], [30, 10, 70, 128]];
    let sources: Vec<_> = colors
        .iter()
        .map(|color| {
            Bitmap::from_pixels(
                PixelFormat::RGBA8888,
                AlphaFormat::Premul,
                &std::iter::repeat_n(*color, 4096)
                    .flatten()
                    .collect::<Vec<_>>(),
                PixelSize::new(64, 64),
                Vector::new(96., 96.),
                256,
            )
        })
        .collect();
    for op in operations {
        let b = bitmap();
        let mut d = b.drawing(false, None);
        assert!(d
            .as_drawing_context_impl_with_layer_effects()
            .unwrap()
            .push_pixel_operation(Rect::new(0., 0., 64., 64.), &op));
        for (i, source) in sources.iter().enumerate() {
            if i > 0 {
                d.as_drawing_context_impl_with_layer_effects()
                    .unwrap()
                    .next_pixel_operation_source();
            }
            d.draw_bitmap(
                source,
                1.,
                Rect::new(0., 0., 64., 64.),
                Rect::new(0., 0., 64., 64.),
            );
        }
        d.as_drawing_context_impl_with_layer_effects()
            .unwrap()
            .pop_pixel_operation();
        d.dispose();
        let p = b.pixels();
        let mut expected: Vec<u8> = std::iter::repeat_n(colors[0], 4096).flatten().collect();
        for color in &colors[1..] {
            op.apply_to_premultiplied_rgba8(
                &mut expected,
                &std::iter::repeat_n(*color, 4096)
                    .flatten()
                    .collect::<Vec<_>>(),
                64,
            );
        }
        for y in [0, 7, 32, 63] {
            for x in [0, 3, 24, 63] {
                let mut e: [u8; 4] = expected[(y * 64 + x) * 4..][..4].try_into().unwrap();
                e.swap(0, 2);
                let out = pixel(&p, x, y);
                for c in 0..4 {
                    assert!(
                        (i32::from(out[c]) - i32::from(e[c])).abs() <= 2,
                        "{op:?} {x},{y}: {out:?} vs {e:?}"
                    );
                }
            }
        }
    }
}
#[test]
fn gpu_lighting_matches_material_attenuation_and_spot_cones() {
    use ferroui_base::{
        platform::LightingParameters,
        rendering::composition::server::{
            LightAttenuation, LightColor, LightMap, LightSource, LightingMaterial,
        },
        Matrix, Vector3D,
    };
    let mut map = LightMap::new();
    map.add(
        LightSource::Ambient {
            color: LightColor::new(0.1, 0.2, 0.3),
        },
        Matrix::IDENTITY,
    );
    let attenuation = LightAttenuation {
        constant: 1.,
        linear: 0.002,
        quadratic: 0.0002,
        min_cutoff: 4.,
        max_cutoff: 80.,
    };
    map.add(
        LightSource::Point {
            color: LightColor::new(0.6, 0.2, 0.1),
            position: Vector3D::new(10., 24., 20.),
            attenuation,
        },
        Matrix::create_translation(2., 3.),
    );
    map.add(
        LightSource::Spot {
            inner_color: LightColor::new(0.1, 0.5, 0.7),
            outer_color: LightColor::new(0.5, 0.1, 0.2),
            position: Vector3D::new(32., 32., 25.),
            direction: Vector3D::new(0., 0., -1.),
            inner_cone_angle: 0.25,
            outer_cone_angle: 1.,
            attenuation,
        },
        Matrix::IDENTITY,
    );
    let lighting = LightingParameters::new(
        &map,
        LightingMaterial {
            ambient: 0.6,
            diffuse: 0.8,
            specular: 0.2,
            shine: 12.,
        },
    );
    for multiply in [false, true] {
        let b = bitmap();
        let mut d = b.drawing(false, None);
        if multiply {
            assert!(d
                .as_drawing_context_impl_with_layer_effects()
                .unwrap()
                .push_lighting_multiply_layer(Rect::new(0., 0., 64., 64.), &lighting));
            rectangle(
                &mut d,
                Color::from_argb(200, 128, 128, 128),
                Rect::new(0., 0., 64., 64.),
            );
            d.as_drawing_context_impl_with_layer_effects()
                .unwrap()
                .pop_multiply_layer();
        } else {
            assert!(d
                .as_drawing_context_impl_with_layer_effects()
                .unwrap()
                .draw_lighting(Rect::new(0., 0., 64., 64.), &lighting));
        }
        d.dispose();
        let p = b.pixels();
        for (x, y) in [(4, 8), (12, 26), (32, 32), (56, 55)] {
            let l = lighting.lit_at(Point::new(x as f64 + 0.5, y as f64 + 0.5));
            let light = [l.b, l.g, l.r];
            let out = pixel(&p, x, y);
            for c in 0..3 {
                let expected = if multiply {
                    (100. * light[c].clamp(0., 8.)).min(200.)
                } else {
                    light[c].clamp(0., 1.) * 255.
                };
                assert!(
                    (f64::from(out[c]) - expected).abs() <= 2.,
                    "multiply={multiply} at {x},{y}: {out:?} expected={light:?}"
                );
            }
            assert_eq!(out[3], if multiply { 200 } else { 255 });
        }
    }
}

#[test]
fn scene_brushes_respect_content_origin_tiling_transform_and_opacity() {
    use ferroui_base::{
        media::{
            immutable::{ImmutableImageBrush, ImmutableTransform},
            AlignmentX, AlignmentY, IBrush, IImmutableBrush, ISceneBrush, ISceneBrushContent,
            ITileBrush, ITransform, Stretch, TileMode,
        },
        Matrix, RelativePoint, RelativeRect, RelativeUnit,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[derive(Clone)]
    struct Scene {
        base: ImmutableImageBrush,
        disposed: Arc<AtomicUsize>,
    }
    impl IBrush for Scene {
        fn opacity(&self) -> f64 {
            self.base.opacity()
        }
        fn transform(&self) -> Option<Rc<dyn ITransform>> {
            IBrush::transform(&self.base)
        }
        fn transform_origin(&self) -> RelativePoint {
            self.base.transform_origin()
        }
        fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
            self.base.relative_transform()
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_scene_brush(&self) -> Option<&dyn ISceneBrush> {
            Some(self)
        }
    }
    impl IImmutableBrush for Scene {}
    impl ITileBrush for Scene {
        fn alignment_x(&self) -> AlignmentX {
            self.base.alignment_x()
        }
        fn alignment_y(&self) -> AlignmentY {
            self.base.alignment_y()
        }
        fn destination_rect(&self) -> RelativeRect {
            self.base.destination_rect()
        }
        fn source_rect(&self) -> RelativeRect {
            self.base.source_rect()
        }
        fn stretch(&self) -> Stretch {
            self.base.stretch()
        }
        fn tile_mode(&self) -> TileMode {
            self.base.tile_mode()
        }
    }
    impl ISceneBrush for Scene {
        fn create_content(&self) -> Option<Rc<dyn ISceneBrushContent>> {
            Some(Rc::new(self.clone()))
        }
    }
    impl ISceneBrushContent for Scene {
        fn brush(&self) -> Rc<dyn ITileBrush> {
            Rc::new(self.base.clone())
        }
        fn rect(&self) -> Rect {
            Rect::new(10., 20., 8., 4.)
        }
        fn use_scalable_rasterization(&self) -> bool {
            true
        }
        fn dispose(&self) {
            self.disposed.fetch_add(1, Ordering::SeqCst);
        }
        fn render(&self, context: &mut dyn IDrawingContextImpl, transform: Option<Matrix>) {
            let old = context.transform();
            context.set_transform(transform.unwrap_or(Matrix::IDENTITY) * old);
            rectangle(context, Colors::RED, Rect::new(10., 20., 4., 4.));
            rectangle(context, Colors::BLUE, Rect::new(14., 20., 4., 4.));
            context.set_transform(old);
        }
    }
    let disposed = Arc::new(AtomicUsize::new(0));
    let scene = Scene {
        base: ImmutableImageBrush::new(
            None,
            AlignmentX::Left,
            AlignmentY::Top,
            Some(RelativeRect::new(0., 0., 16., 8., RelativeUnit::Absolute)),
            0.5,
            Some(Rc::new(ImmutableTransform::new(
                Matrix::create_translation(4., 3.),
            ))),
            RelativePoint::TOP_LEFT,
            None,
            Stretch::Fill,
            TileMode::FlipX,
            None,
        ),
        disposed: disposed.clone(),
    };
    let b = bitmap();
    let mut d = b.drawing(false, None);
    d.draw_rectangle(
        Some(&scene),
        None,
        RoundedRect::from_rect(Rect::new(0., 0., 32., 32.)),
        &Default::default(),
    );
    d.dispose();
    let p = b.pixels();
    let a = pixel(&p, 6, 5);
    let b = pixel(&p, 22, 5);
    assert_eq!(a, [0, 0, 128, 128]);
    assert_eq!(b, [128, 0, 0, 128]);
    assert_eq!(disposed.load(Ordering::SeqCst), 1);
}
#[test]
fn acrylic_material_composes_tint_noise_and_digger_alpha_on_gpu() {
    use ferroui_base::media::{AcrylicBackgroundSource, IExperimentalAcrylicMaterial};
    struct Material(bool);
    impl IExperimentalAcrylicMaterial for Material {
        fn background_source(&self) -> AcrylicBackgroundSource {
            if self.0 {
                AcrylicBackgroundSource::Digger
            } else {
                AcrylicBackgroundSource::None
            }
        }
        fn tint_color(&self) -> Color {
            Color::from_argb(128, 255, 0, 0)
        }
        fn tint_opacity(&self) -> f64 {
            0.5
        }
        fn material_color(&self) -> Color {
            Color::from_argb(128, 0, 0, 255)
        }
        fn fallback_color(&self) -> Color {
            Colors::WHITE
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }
    for replace in [false, true] {
        let b = bitmap();
        let mut d = b.drawing(false, None);
        d.clear(Colors::GREEN);
        d.as_drawing_context_with_acrylic_like_support()
            .unwrap()
            .draw_rectangle_with_material(
                &Material(replace),
                RoundedRect::from_rect(Rect::new(4., 4., 20., 20.)),
            );
        d.dispose();
        let p = b.pixels();
        let out = pixel(&p, 10, 10);
        assert!(
            out[2] >= 120 && out[2] < 140 && out[0] >= 55 && out[0] < 75,
            "{out:?}"
        );
        if replace {
            assert!(out[3] >= 190 && out[3] < 202);
            assert!(out[1] < 8);
        } else {
            assert_eq!(out[3], 255);
            assert!(out[1] > 25 && out[1] < 40);
        }
        assert_eq!(pixel(&p, 1, 1), [0, 128, 0, 255]);
    }
}

#[test]
fn spatially_binned_paths_keep_empty_rows_transparent_and_preserve_winding() {
    use kurbo::{BezPath, Rect as KRect, Shape};
    let mut path = BezPath::new();
    for x in 0..32 {
        for y in [1., 58.] {
            path.extend(
                KRect::new((x * 2) as f64, y, (x * 2 + 1) as f64, y + 2.)
                    .to_path(0.1)
                    .elements()
                    .iter()
                    .copied(),
            );
        }
    }
    let geometry = Geometry::new(path, FillRule::EvenOdd);
    for _ in 0..2 {
        let b = bitmap();
        let mut d = b.drawing(false, None);
        d.draw_geometry(
            Some(&ImmutableSolidColorBrush::new(Colors::RED)),
            None,
            &*geometry,
        );
        d.dispose();
        let p = b.pixels();
        assert_eq!(pixel(&p, 0, 1), [0, 0, 255, 255]);
        assert_eq!(pixel(&p, 1, 1), [0; 4]);
        assert_eq!(pixel(&p, 0, 32), [0; 4]);
        assert_eq!(pixel(&p, 60, 59), [0, 0, 255, 255]);
    }
}

#[test]
fn batches_and_texture_pool_bound_submissions_and_retain_snapshot_aliases() {
    const FLAG: &str = "FERRO_QUARTZ_ISOLATED_POOL_TEST";
    if std::env::var_os(FLAG).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "effects_tests::batches_and_texture_pool_bound_submissions_and_retain_snapshot_aliases", "--nocapture"])
            .env(FLAG, "1").status().unwrap();
        assert!(status.success());
        return;
    }
    fn counters() -> [u64; 9] {
        let mut c = [0; 9];
        unsafe { ffi::qm_statistics(c.as_mut_ptr(), c.len()) };
        c
    }
    let start = counters();
    for i in 0..64 {
        let b = bitmap();
        let mut d = b.drawing(false, None);
        rectangle(
            &mut d,
            if i % 2 == 0 {
                Colors::RED
            } else {
                Colors::BLUE
            },
            Rect::new(0., 0., 64., 64.),
        );
        d.dispose();
    }
    let end = counters();
    assert!(
        end[7] - start[7] < 8,
        "command buffers: {start:?} -> {end:?}"
    );
    assert!(
        end[8] - start[8] <= 2,
        "texture allocations: {start:?} -> {end:?}"
    );
    let b = bitmap();
    let mut d = b.drawing(false, None);
    rectangle(&mut d, Colors::RED, Rect::new(0., 0., 64., 64.));
    d.dispose();
    let alias = b.context(|p| unsafe { ffi::Owned::new(ffi::qm_layer_snapshot(p)) });
    drop(b);
    for _ in 0..8 {
        let b = bitmap();
        b.write(&vec![0; 64 * 64 * 4]);
    }
    let mut data = vec![0; 64 * 64 * 4];
    unsafe { ffi::q_read_pixels(alias.0, data.as_mut_ptr(), data.len()) };
    assert_eq!(pixel(&data, 32, 32), [0, 0, 255, 255]);
    assert_eq!(counters()[6], 0);
}

#[test]
fn mipmapped_sampling_averages_minification_and_invalidates_after_source_writes() {
    use ferroui_base::media::imaging::BitmapInterpolationMode as Mode;
    let source = bitmap();
    let pixels: Vec<u8> = (0..64 * 64)
        .flat_map(|i| {
            if (i % 64 + i / 64) % 2 == 0 {
                [255; 4]
            } else {
                [0, 0, 0, 255]
            }
        })
        .collect();
    source.write(&pixels);
    for mode in [Mode::MediumQuality, Mode::HighQuality] {
        let small = Bitmap::resized(&source, PixelSize::new(5, 5), mode);
        for p in small.pixels().chunks_exact(4) {
            assert!((i32::from(p[0]) - 128).abs() <= 2, "{mode:?}: {p:?}");
            assert_eq!(p[3], 255);
        }
    }
    source.write(&vec![255; 64 * 64 * 4]);
    let small = Bitmap::resized(&source, PixelSize::new(5, 5), Mode::MediumQuality);
    assert!(small.pixels().iter().all(|x| *x == 255));
    let nearest = Bitmap::resized(&source, PixelSize::new(128, 128), Mode::None);
    let cubic = Bitmap::resized(&source, PixelSize::new(128, 128), Mode::HighQuality);
    assert_eq!(nearest.pixels(), cubic.pixels());
}
#[cfg(feature = "runtime-shaders")]
#[test]
fn native_shader_passes_match_pixel_references_at_translated_bounds() {
    use ferroui_base::media::effects::{library::*, ImmutableCustomEffect};
    let effects = [
        ImmutableCustomEffect::new(PixelateEffect::default()),
        ImmutableCustomEffect::new(ScanlineShaderEffect::default()),
        ImmutableCustomEffect::new(SpotlightShaderEffect::default()),
    ];
    for effect in effects {
        let source = Bitmap::new(PixelSize::new(32, 32), Vector::new(96., 96.));
        let mut rgba: Vec<u8> = (0..1024)
            .flat_map(|i| [(i % 32 * 7) as u8, (i / 32 * 7) as u8, 100, 255])
            .collect();
        let mut bgra = rgba.clone();
        for p in bgra.chunks_exact_mut(4) {
            p.swap(0, 2)
        }
        source.write(&bgra);
        assert!(effect.cpu_reference(&mut rgba, 32, 32, 1.));
        let b = bitmap();
        let mut d = b.drawing(false, None);
        let area = Rect::new(8., 12., 32., 32.);
        d.as_drawing_context_impl_with_effects()
            .unwrap()
            .push_effect(Some(area), &effect);
        d.draw_bitmap(&source, 1., Rect::new(0., 0., 32., 32.), area);
        d.as_drawing_context_impl_with_effects()
            .unwrap()
            .pop_effect();
        d.dispose();
        let pixels = b.pixels();
        for y in 0..32 {
            for x in 0..32 {
                let actual = pixel(&pixels, x + 8, y + 12);
                let expected = &rgba[(y * 32 + x) * 4..][..4];
                for c in 0..4 {
                    let channel = if c == 0 {
                        2
                    } else if c == 2 {
                        0
                    } else {
                        c
                    };
                    assert!(
                        (i32::from(actual[c]) - i32::from(expected[channel])).abs() <= 3,
                        "{} at {x},{y}: {actual:?} vs {expected:?}",
                        effect.name()
                    );
                }
            }
        }
        assert_eq!(pixel(&pixels, 0, 0), [0; 4]);
    }
}
