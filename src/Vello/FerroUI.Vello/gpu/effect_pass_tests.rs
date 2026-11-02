//! The render passes of `effect_passes.rs` in the GPU modes: the lighting
//! of the compositor drawn by the device, compared with the exact light (the
//! light of the compositor in double precision at the point each pixel
//! shows, rounded to 8 bits), and the frames of a scroll allocating nothing
//! once the pool holds the textures they need; the layers with an effect
//! (the colour matrix, the multiply layer of lit visuals, the pixel
//! operations, the convolution, the luminance mask) drawn on the device,
//! compared with their CPU references applied to the same content drawn
//! plain in the same mode, without a layer drawn on the processor, and their
//! steady frames allocating nothing.
//!
//! The pass evaluates the same math in single precision per pixel and the
//! scene paints its texture one texel per device pixel: within 1 level of
//! the exact light (2 on a device whose `pow` is approximate). The edge of
//! the area (2 device pixels, anti-aliased) is left out.
//!
//! Skipped with a message on a machine without a graphics adapter.

use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::gpu::VelloWgpuDevice;
use crate::scene::IVelloSceneSink;
use crate::vello_options::VelloRenderingMode;
use ferroui_base::platform::{IDrawingContextImpl, LightingParameters};
use ferroui_base::rendering::composition::server::{
    LightAttenuation, LightColor, LightMap, LightSource, LightingMaterial,
};
use ferroui_base::{Matrix, Point, Rect, Vector, Vector3D};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

const SIZE: u16 = 96;

fn device_modes(device: &Arc<VelloWgpuDevice>) -> Vec<VelloRenderingMode> {
    let mut modes = vec![VelloRenderingMode::Hybrid];
    if cfg!(feature = "gpu") && device.supports_compute() {
        modes.push(VelloRenderingMode::Gpu);
    }
    modes
}

fn context(device: &Arc<VelloWgpuDevice>, mode: VelloRenderingMode, pixels: &Rc<RefCell<Vec<u8>>>) -> DrawingContextImpl {
    let sink: Box<dyn IVelloSceneSink> =
        crate::gpu::create_window_scene_sink(device, &[mode], SIZE, SIZE, wgpu::TextureFormat::Rgba8Unorm);
    let pixels = pixels.clone();
    DrawingContextImpl::new(
        CreateInfo {
            sink,
            backdrop: None,
            on_finished: Box::new(move |sink| sink.render_to_pixels(&mut pixels.borrow_mut())),
            scale_drawing_to_dpi: false,
            dpi: Vector::new(96.0, 96.0),
            rendering_modes: vec![mode],
        },
        Vec::new(),
    )
}

fn lights(frame: f64) -> LightMap {
    let mut map = LightMap::new();
    map.add(LightSource::Ambient { color: LightColor::new(0.05, 0.05, 0.1) }, Matrix::IDENTITY);
    let attenuation = LightAttenuation { constant: 1.0, linear: 0.004, quadratic: 0.0, min_cutoff: 10.0, max_cutoff: f64::MAX };
    map.add(
        LightSource::Point {
            color: LightColor::new(0.8, 0.5, 0.3),
            position: Vector3D::new(30.0, 25.0 + frame, 40.0),
            attenuation,
        },
        Matrix::IDENTITY,
    );
    map.add(
        LightSource::Spot {
            inner_color: LightColor::new(1.0, 0.9, 0.7),
            outer_color: LightColor::new(0.2, 0.3, 1.0),
            position: Vector3D::new(20.0, 20.0, 60.0),
            direction: Vector3D::new(0.5, 0.4, -1.0),
            inner_cone_angle: 0.25,
            outer_cone_angle: 0.6,
            attenuation,
        },
        Matrix::create_translation(-5.0, 3.0),
    );
    map
}

const SHINY: LightingMaterial = LightingMaterial { ambient: 0.5, diffuse: 0.7, specular: 0.6, shine: 24.0 };

#[test]
fn the_device_draws_the_exact_light_and_steady_frames_allocate_nothing() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    let rect = Rect::new(0.0, 0.0, 70.0, 60.0);
    for mode in device_modes(&device) {
        let mut allocations_after_first = None;
        for frame in 0..6 {
            let transform = Matrix::create_rotation(0.3) * Matrix::create_translation(20.0, 2.0 - frame as f64);
            let map = lights(frame as f64);
            let lighting = LightingParameters::new(&map, SHINY);
            let pixels = Rc::new(RefCell::new(vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4]));
            let mut drawing = context(&device, mode, &pixels);
            drawing.set_transform(transform);
            let layers = drawing.as_drawing_context_impl_with_layer_effects().unwrap();
            assert!(layers.draw_lighting(rect, &lighting), "{mode:?}: the device draws the lights");
            drawing.dispose();
            if frame == 0 {
                let pixels = pixels.borrow();
                let inverse = transform.try_invert().unwrap();
                let inner = Rect::new(2.0, 2.0, rect.width - 4.0, rect.height - 4.0);
                let (mut largest, mut compared) = (0, 0);
                for y in 0..usize::from(SIZE) {
                    for x in 0..usize::from(SIZE) {
                        let local = inverse.transform(Point::new(x as f64 + 0.5, y as f64 + 0.5));
                        if !inner.contains(local) {
                            continue;
                        }
                        compared += 1;
                        let light = lighting.lit_at(local).clamped();
                        let expected = [light.r, light.g, light.b].map(|value| (value * 255.0).round() as i32);
                        let at = (y * usize::from(SIZE) + x) * 4;
                        for channel in 0..3 {
                            largest = largest.max((expected[channel] - i32::from(pixels[at + channel])).abs());
                        }
                        largest = largest.max((255 - i32::from(pixels[at + 3])).abs());
                    }
                }
                assert!(compared > 2000, "{mode:?}: {compared}");
                assert!(largest <= 2, "{mode:?}: within 2 levels of the exact light: {largest}");
            }
            // The first frames fill the pool; the next allocate nothing.
            let allocations = super::effect_passes::texture_allocations();
            if frame == 2 {
                allocations_after_first = Some(allocations);
            } else if let Some(previous) = allocations_after_first {
                assert_eq!(previous, allocations, "{mode:?}: frame {frame} allocated a texture");
            }
        }
    }
}

/// The custom effects FerroUI ships, drawn as their graphs in the hybrid
/// and GPU modes (#455, E2 and E3), compared with the CPU reference of each
/// effect applied to the same scene drawn plain in the same mode. The colour
/// matrix and convolution layers take the route of the device ("layers with
/// an effect", gpu-rendering.md): no layer is drawn on the processor.
#[test]
fn the_library_effects_match_their_cpu_references_in_the_gpu_modes() {
    use ferroui_base::media::effects::library::{
        BrightnessContrastEffect, EdgeDetectEffect, GlowEffect, GrayscaleEffect, InvertEffect, SaturationEffect,
        SepiaEffect, SharpenEffect, TintEffect,
    };
    use ferroui_base::media::effects::ImmutableCustomEffect;
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::{BoxShadows, Color, Colors, EdgeMode, RenderOptions};
    use ferroui_base::rendering::composition::effects::{pop_custom_effect_graph, push_custom_effect_graph};
    use ferroui_base::RoundedRect;

    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    fn scene(context: &mut dyn IDrawingContextImpl) {
        let solid = |color: Color| ImmutableSolidColorBrush::new(color);
        let square = |x: f64, y: f64, w: f64, h: f64| RoundedRect::from_rect(Rect::new(x, y, w, h));
        context.push_render_options(RenderOptions { edge_mode: EdgeMode::Aliased, ..RenderOptions::default() });
        context.draw_rectangle(Some(&solid(Colors::RED)), None, square(24.0, 24.0, 20.0, 20.0), &BoxShadows::default());
        let steel_blue = Color::from_rgb(48, 128, 192);
        context.draw_rectangle(Some(&solid(steel_blue)), None, square(46.0, 24.0, 20.0, 20.0), &BoxShadows::default());
        let orange = Color::from_argb(128, 255, 128, 0);
        context.draw_rectangle(Some(&solid(orange)), None, square(24.0, 46.0, 42.0, 18.0), &BoxShadows::default());
        context.pop_render_options();
    }
    let render = |mode: VelloRenderingMode, effect: Option<&ImmutableCustomEffect>| -> Vec<u8> {
        let pixels = Rc::new(RefCell::new(vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4]));
        let mut drawing = context(&device, mode, &pixels);
        let bounds = Rect::new(0.0, 0.0, f64::from(SIZE), f64::from(SIZE));
        if let Some(effect) = effect {
            assert!(push_custom_effect_graph(&mut drawing, effect, bounds));
        }
        scene(&mut drawing);
        if let Some(effect) = effect {
            assert!(pop_custom_effect_graph(&mut drawing, effect, bounds));
        }
        drawing.dispose();
        let rendered = pixels.borrow().clone();
        rendered
    };

    let effects = [
        (ImmutableCustomEffect::new(TintEffect { color: Colors::TEAL, strength: 0.6 }), 2u8),
        (ImmutableCustomEffect::new(GrayscaleEffect { amount: 0.8 }), 2),
        (ImmutableCustomEffect::new(SepiaEffect { amount: 0.9 }), 2),
        (ImmutableCustomEffect::new(SaturationEffect { saturation: 1.7 }), 2),
        (ImmutableCustomEffect::new(BrightnessContrastEffect { brightness: 0.1, contrast: 1.4 }), 2),
        (ImmutableCustomEffect::new(InvertEffect { amount: 1.0 }), 2),
        (ImmutableCustomEffect::new(GlowEffect { color: Colors::BLUE, blur_radius: 4.0, intensity: 0.8 }), 12),
        (ImmutableCustomEffect::new(SharpenEffect { strength: 0.8 }), 2),
        (ImmutableCustomEffect::new(EdgeDetectEffect { strength: 1.0 }), 2),
    ];
    for mode in device_modes(&device) {
        for (effect, largest) in &effects {
            let mut expected = render(mode, None);
            assert!(effect.cpu_reference(&mut expected, usize::from(SIZE), usize::from(SIZE), 1.0), "{effect}");
            let cpu_layers = crate::drawing_context_impl::cpu_layer_count();
            let actual = render(mode, Some(effect));
            assert_eq!(
                cpu_layers,
                crate::drawing_context_impl::cpu_layer_count(),
                "{mode:?}: {effect}: a layer was drawn on the processor"
            );
            let most = expected.iter().zip(&actual).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
            let total: u64 = expected.iter().zip(&actual).map(|(a, b)| u64::from(a.abs_diff(*b))).sum();
            let mean = total as f64 / expected.len() as f64;
            assert!(most <= *largest && mean <= 0.5, "{mode:?}: {effect}: largest difference {most}, mean {mean}");
        }
    }
}

// --- layers with an effect ----------------------------------------------------------------

use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{BoxShadows, Color, ColorMatrix, ColorMatrixAlphaMode, Colors, ConvolveMatrix};
use ferroui_base::media::{EdgeMode, RenderOptions};
use ferroui_base::platform::PixelOperation;
use ferroui_base::rendering::composition::effects::BlendEffectMode;
use ferroui_base::{PixelSize, RoundedRect};

/// Opaque red and steel blue squares and a translucent orange band, with
/// aliased edges.
fn content(context: &mut dyn IDrawingContextImpl) {
    let solid = |color: Color| ImmutableSolidColorBrush::new(color);
    let square = |x: f64, y: f64, w: f64, h: f64| RoundedRect::from_rect(Rect::new(x, y, w, h));
    context.push_render_options(RenderOptions { edge_mode: EdgeMode::Aliased, ..RenderOptions::default() });
    context.draw_rectangle(Some(&solid(Colors::RED)), None, square(24.0, 24.0, 20.0, 20.0), &BoxShadows::default());
    let steel_blue = Color::from_rgb(48, 128, 192);
    context.draw_rectangle(Some(&solid(steel_blue)), None, square(46.0, 24.0, 20.0, 20.0), &BoxShadows::default());
    let orange = Color::from_argb(128, 255, 128, 0);
    context.draw_rectangle(Some(&solid(orange)), None, square(24.0, 46.0, 42.0, 18.0), &BoxShadows::default());
    context.pop_render_options();
}

/// A second source of the pixel operations: a translucent green and an
/// opaque yellow square over part of the content.
fn other_content(context: &mut dyn IDrawingContextImpl) {
    let solid = |color: Color| ImmutableSolidColorBrush::new(color);
    let square = |x: f64, y: f64, w: f64, h: f64| RoundedRect::from_rect(Rect::new(x, y, w, h));
    context.push_render_options(RenderOptions { edge_mode: EdgeMode::Aliased, ..RenderOptions::default() });
    let green = Color::from_argb(160, 64, 200, 100);
    context.draw_rectangle(Some(&solid(green)), None, square(16.0, 16.0, 40.0, 30.0), &BoxShadows::default());
    let yellow = Color::from_rgb(230, 210, 40);
    context.draw_rectangle(Some(&solid(yellow)), None, square(40.0, 40.0, 40.0, 40.0), &BoxShadows::default());
    context.pop_render_options();
}

/// The premultiplied RGBA pixels of `draw` drawn in `mode`.
fn draw_in(device: &Arc<VelloWgpuDevice>, mode: VelloRenderingMode, draw: impl FnOnce(&mut DrawingContextImpl)) -> Vec<u8> {
    let pixels = Rc::new(RefCell::new(vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4]));
    let mut drawing = context(device, mode, &pixels);
    draw(&mut drawing);
    drawing.dispose();
    let rendered = pixels.borrow().clone();
    rendered
}

/// The largest difference of a channel, the mean difference, and the
/// number of pixels that differ by more than 2 levels.
fn difference(expected: &[u8], actual: &[u8]) -> (u8, f64, usize) {
    assert_eq!(expected.len(), actual.len());
    let largest = expected.iter().zip(actual).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
    let total: u64 = expected.iter().zip(actual).map(|(a, b)| u64::from(a.abs_diff(*b))).sum();
    let pixels = expected
        .chunks_exact(4)
        .zip(actual.chunks_exact(4))
        .filter(|(a, b)| a.iter().zip(b.iter()).any(|(a, b)| a.abs_diff(*b) > 2))
        .count();
    (largest, total as f64 / expected.len() as f64, pixels)
}

/// The whole target.
fn full() -> Rect {
    Rect::new(0.0, 0.0, f64::from(SIZE), f64::from(SIZE))
}

/// The colour matrix layers, the convolution layers and the luminance mask
/// on the device, against the CPU references (`ColorMatrix::apply_to_rgba8`,
/// `ConvolveMatrix::apply_to_premultiplied_rgba8`, the luminance of the CPU
/// path) applied to the content drawn plain in the same mode: within 2
/// levels, and no layer drawn on the processor.
#[test]
fn the_layer_passes_match_their_cpu_references_in_the_gpu_modes() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    let size = usize::from(SIZE);
    // A matrix with offsets: what is transparent inside the bounds shows.
    let tinted = ColorMatrix::new([
        [0.5, 0.2, 0.1, 0.0],
        [0.3, 0.6, 0.0, 0.0],
        [0.1, 0.1, 0.8, 0.0],
        [0.0, 0.0, 0.0, 0.9],
        [0.05, 0.0, 0.1, 0.1],
    ]);
    let mut shifted = ConvolveMatrix::new(3, 3, &[1.0, 2.0, 0.0, 0.0, 1.0, -1.0, 0.0, 0.0, 0.5]).unwrap();
    shifted.divisor = 3.5;
    shifted.bias = 0.05;
    let mut hard = ConvolveMatrix::new(2, 3, &[0.5, 0.0, 0.0, 1.0, -0.25, 0.75]).unwrap();
    hard.clamp_edges = true;
    hard.preserve_alpha = true;
    // Part of the content, on whole pixels.
    let (x0, y0, width, height) = (20usize, 22usize, 40usize, 30usize);
    let part = Rect::new(x0 as f64, y0 as f64, width as f64, height as f64);

    for mode in device_modes(&device) {
        let plain = draw_in(&device, mode, |drawing| content(drawing));
        let cpu_layers = crate::drawing_context_impl::cpu_layer_count();

        for alpha_mode in [ColorMatrixAlphaMode::Premultiplied, ColorMatrixAlphaMode::Straight] {
            let mut expected = plain.clone();
            tinted.apply_to_rgba8(&mut expected, alpha_mode);
            let actual = draw_in(&device, mode, |drawing| {
                let layers = drawing.as_drawing_context_impl_with_layer_effects().unwrap();
                layers.push_color_matrix_layer(full(), &tinted, alpha_mode);
                content(drawing);
                drawing.as_drawing_context_impl_with_layer_effects().unwrap().pop_color_matrix_layer();
            });
            let (most, mean, _) = difference(&expected, &actual);
            assert!(most <= 2 && mean <= 0.5, "{mode:?}: colour matrix {alpha_mode:?}: largest {most}, mean {mean}");
        }

        let mut expected = plain.clone();
        shifted.apply_to_premultiplied_rgba8(&mut expected, size, size);
        let actual = draw_in(&device, mode, |drawing| {
            drawing.as_drawing_context_impl_with_layer_effects().unwrap().push_convolve_matrix_layer(full(), &shifted);
            content(drawing);
            drawing.as_drawing_context_impl_with_layer_effects().unwrap().pop_convolve_matrix_layer();
        });
        let (most, mean, _) = difference(&expected, &actual);
        assert!(most <= 2 && mean <= 0.5, "{mode:?}: convolution: largest {most}, mean {mean}");

        // The hard edges are those of the bounds; nothing outside them.
        let mut area = Vec::with_capacity(width * height * 4);
        for y in y0..y0 + height {
            area.extend_from_slice(&plain[(y * size + x0) * 4..(y * size + x0 + width) * 4]);
        }
        hard.apply_to_premultiplied_rgba8(&mut area, width, height);
        let mut expected = vec![0u8; plain.len()];
        for y in 0..height {
            let at = ((y0 + y) * size + x0) * 4;
            expected[at..at + width * 4].copy_from_slice(&area[y * width * 4..(y + 1) * width * 4]);
        }
        let actual = draw_in(&device, mode, |drawing| {
            drawing.as_drawing_context_impl_with_layer_effects().unwrap().push_convolve_matrix_layer(part, &hard);
            content(drawing);
            drawing.as_drawing_context_impl_with_layer_effects().unwrap().pop_convolve_matrix_layer();
        });
        let (most, mean, _) = difference(&expected, &actual);
        assert!(most <= 2 && mean <= 0.5, "{mode:?}: convolution with hard edges: largest {most}, mean {mean}");

        // The luminance mask: black, the luminance of the colour as the alpha.
        let mut expected = plain.clone();
        for pixel in expected.chunks_exact_mut(4) {
            let alpha = (0.2126 * f64::from(pixel[0]) + 0.7152 * f64::from(pixel[1]) + 0.0722 * f64::from(pixel[2]))
                .round()
                .clamp(0.0, 255.0) as u8;
            pixel.copy_from_slice(&[0, 0, 0, alpha]);
        }
        let actual = draw_in(&device, mode, |drawing| {
            let effects = drawing.as_drawing_context_impl_with_effects().unwrap();
            effects.push_effect(Some(full()), &ferroui_base::media::ImmutableLuminanceToAlphaEffect);
            content(drawing);
            drawing.as_drawing_context_impl_with_effects().unwrap().pop_effect();
        });
        let (most, mean, _) = difference(&expected, &actual);
        assert!(most <= 2 && mean <= 0.5, "{mode:?}: luminance mask: largest {most}, mean {mean}");

        assert_eq!(cpu_layers, crate::drawing_context_impl::cpu_layer_count(), "{mode:?}: a layer was drawn on the processor");
    }
}

/// The multiply layer of lit visuals on the device: the content times the
/// light of the compositor in double precision at each pixel, clamped to
/// 0..16, the colour clamped to the alpha.
#[test]
fn the_multiply_layer_of_lit_visuals_matches_the_exact_light_in_the_gpu_modes() {
    use ferroui_base::rendering::composition::server::LIGHT_MAP_MAX_SCALE;
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    let map = lights(3.0);
    let lighting = LightingParameters::new(&map, SHINY);
    for mode in device_modes(&device) {
        let plain = draw_in(&device, mode, |drawing| content(drawing));
        let mut expected = plain.clone();
        for (index, pixel) in expected.chunks_exact_mut(4).enumerate() {
            let (x, y) = ((index % usize::from(SIZE)) as f64 + 0.5, (index / usize::from(SIZE)) as f64 + 0.5);
            let light = lighting.lit_at(Point::new(x, y));
            let alpha = f64::from(pixel[3]);
            for (channel, factor) in [light.r, light.g, light.b].into_iter().enumerate() {
                let factor = factor.clamp(0.0, LIGHT_MAP_MAX_SCALE);
                pixel[channel] = (f64::from(pixel[channel]) * factor).min(alpha).round() as u8;
            }
        }
        let cpu_layers = crate::drawing_context_impl::cpu_layer_count();
        let actual = draw_in(&device, mode, |drawing| {
            let layers = drawing.as_drawing_context_impl_with_layer_effects().unwrap();
            assert!(layers.push_lighting_multiply_layer(full(), &lighting), "{mode:?}: the device multiplies by the lights");
            content(drawing);
            drawing.as_drawing_context_impl_with_layer_effects().unwrap().pop_multiply_layer();
        });
        assert_eq!(cpu_layers, crate::drawing_context_impl::cpu_layer_count(), "{mode:?}: drawn on the processor");
        let (most, mean, _) = difference(&expected, &actual);
        assert!(most <= 2 && mean <= 0.5, "{mode:?}: lit content: largest {most}, mean {mean}");
    }
}

/// The pixel operations on the device: two sources drawn apart and
/// combined by the pass, against `PixelOperation::apply_to_premultiplied_rgba8`
/// of the two sources drawn plain in the same mode. `Dissolve` compares a
/// random number with the alpha at each pixel: a few pixels where the two
/// are within a rounding of each other may differ.
#[test]
fn the_pixel_operations_match_their_cpu_reference_in_the_gpu_modes() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    let grid = full();
    let cells = PixelSize::new(i32::from(SIZE), i32::from(SIZE));
    let blend = |mode: BlendEffectMode| PixelOperation::Blend { mode, noise_grid: grid, noise_size: cells };
    let operations = [
        blend(BlendEffectMode::Subtract),
        blend(BlendEffectMode::LinearBurn),
        blend(BlendEffectMode::DarkerColor),
        blend(BlendEffectMode::LighterColor),
        blend(BlendEffectMode::VividLight),
        blend(BlendEffectMode::LinearLight),
        blend(BlendEffectMode::PinLight),
        blend(BlendEffectMode::HardMix),
        blend(BlendEffectMode::Division),
        blend(BlendEffectMode::Dissolve),
        PixelOperation::Arithmetic { k: [0.5, 0.25, 0.5, 0.05] },
        PixelOperation::MaskInvert,
    ];
    for mode in device_modes(&device) {
        let first = draw_in(&device, mode, |drawing| content(drawing));
        let second = draw_in(&device, mode, |drawing| other_content(drawing));
        let cpu_layers = crate::drawing_context_impl::cpu_layer_count();
        for operation in &operations {
            let mut expected = first.clone();
            operation.apply_to_premultiplied_rgba8(&mut expected, &second, usize::from(SIZE));
            let actual = draw_in(&device, mode, |drawing| {
                let layers = drawing.as_drawing_context_impl_with_layer_effects().unwrap();
                assert!(layers.push_pixel_operation(full(), operation), "{mode:?}: {operation:?} on the device");
                content(drawing);
                drawing.as_drawing_context_impl_with_layer_effects().unwrap().next_pixel_operation_source();
                other_content(drawing);
                drawing.as_drawing_context_impl_with_layer_effects().unwrap().pop_pixel_operation();
            });
            let (most, mean, pixels) = difference(&expected, &actual);
            if matches!(operation, PixelOperation::Blend { mode: BlendEffectMode::Dissolve, .. }) {
                assert!(pixels <= 8 && mean <= 0.5, "{mode:?}: {operation:?}: {pixels} pixels differ, mean {mean}");
            } else {
                assert!(most <= 2 && mean <= 0.5, "{mode:?}: {operation:?}: largest {most}, mean {mean}");
            }
        }
        assert_eq!(cpu_layers, crate::drawing_context_impl::cpu_layer_count(), "{mode:?}: drawn on the processor");
    }
}

/// Frames of nested layers with an effect whose parameters change (a
/// colour matrix around a convolution, a lit multiply layer, a pixel
/// operation) take their textures from the pool: once the first frames
/// filled it, no texture is allocated.
#[test]
fn steady_frames_of_the_layers_allocate_no_texture() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    let grid = full();
    let cells = PixelSize::new(i32::from(SIZE), i32::from(SIZE));
    for mode in device_modes(&device) {
        let mut allocations_after_first = None;
        for frame in 0..6 {
            let amount = 0.2 + 0.1 * frame as f32;
            let matrix = ColorMatrix::saturation(1.0 - amount);
            let sharpen =
                ConvolveMatrix::new(3, 3, &[0.0, -amount, 0.0, -amount, 1.0 + 4.0 * amount, -amount, 0.0, -amount, 0.0])
                    .unwrap();
            let map = lights(frame as f64);
            let lighting = LightingParameters::new(&map, SHINY);
            let operation = PixelOperation::Blend { mode: BlendEffectMode::Subtract, noise_grid: grid, noise_size: cells };
            // The bounds move by a fraction of a pixel: the sizes of the pool
            // stay the same.
            let bounds = Rect::new(4.0 + 0.25 * frame as f64, 6.0, 80.0, 70.0);
            let cpu_layers = crate::drawing_context_impl::cpu_layer_count();
            draw_in(&device, mode, |drawing| {
                let layers = drawing.as_drawing_context_impl_with_layer_effects().unwrap();
                layers.push_color_matrix_layer(bounds, &matrix, ColorMatrixAlphaMode::Premultiplied);
                drawing.as_drawing_context_impl_with_layer_effects().unwrap().push_convolve_matrix_layer(bounds, &sharpen);
                content(drawing);
                drawing.as_drawing_context_impl_with_layer_effects().unwrap().pop_convolve_matrix_layer();
                drawing.as_drawing_context_impl_with_layer_effects().unwrap().pop_color_matrix_layer();

                let layers = drawing.as_drawing_context_impl_with_layer_effects().unwrap();
                assert!(layers.push_lighting_multiply_layer(bounds, &lighting));
                other_content(drawing);
                drawing.as_drawing_context_impl_with_layer_effects().unwrap().pop_multiply_layer();

                let layers = drawing.as_drawing_context_impl_with_layer_effects().unwrap();
                assert!(layers.push_pixel_operation(bounds, &operation));
                content(drawing);
                drawing.as_drawing_context_impl_with_layer_effects().unwrap().next_pixel_operation_source();
                other_content(drawing);
                drawing.as_drawing_context_impl_with_layer_effects().unwrap().pop_pixel_operation();
            });
            assert_eq!(cpu_layers, crate::drawing_context_impl::cpu_layer_count(), "{mode:?}: drawn on the processor");
            let allocations = super::effect_passes::texture_allocations();
            if frame == 2 {
                allocations_after_first = Some(allocations);
            } else if let Some(previous) = allocations_after_first {
                assert_eq!(previous, allocations, "{mode:?}: frame {frame} allocated a texture");
            }
        }
    }
}
