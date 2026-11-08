//! The render passes of `effect_passes.rs` in the GPU modes: the lighting
//! of the compositor drawn by the device, compared with the exact light (the
//! light of the compositor in double precision at the point each pixel
//! shows, rounded to 8 bits), and the frames of a scroll allocating nothing
//! once the pool holds the textures they need; the layers with an effect
//! (the colour matrix, the multiply layer of lit visuals, the pixel
//! operations, the convolution, the luminance mask) drawn on the device,
//! compared with their CPU references applied to the same content drawn
//! plain in the same mode, without a layer drawn on the processor, and their
//! steady frames allocating nothing; and the runtime shader effects (#455,
//! E5): their WGSL modules checked by naga against the uniform convention
//! (without a device), the library's effects drawn by their passes against
//! their CPU references, a module that fails drawing the content unchanged,
//! reported once, and their steady frames allocating nothing.
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

/// The graph effects of the library in the hybrid and GPU modes at two
/// device pixels per unit (#455, E9): under a transform of scale 2 over the
/// whole target, the content at half its size, against their CPU references at that scale applied to the
/// content drawn plain in the same mode, with the tolerances of scale 1; no
/// layer drawn on the processor. The colour matrices do not depend on the
/// scale, the blur of the glow is twice as wide, and the kernels of the
/// convolutions step over device pixels.
#[test]
fn the_library_effects_match_their_cpu_references_at_twice_the_scale_in_the_gpu_modes() {
    use ferroui_base::media::effects::library::{
        BrightnessContrastEffect, EdgeDetectEffect, GlowEffect, GrayscaleEffect, InvertEffect, SaturationEffect,
        SepiaEffect, SharpenEffect, TintEffect,
    };
    use ferroui_base::rendering::composition::effects::{pop_custom_effect_graph, push_custom_effect_graph};

    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
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
    // The content at half its size: under the scale it covers the device
    // pixels `content` covers at scale 1, inside the target, so that no
    // content beyond the target reaches the glow.
    fn small_content(context: &mut dyn IDrawingContextImpl) {
        let solid = |color: Color| ImmutableSolidColorBrush::new(color);
        let square = |x: f64, y: f64, w: f64, h: f64| RoundedRect::from_rect(Rect::new(x, y, w, h));
        context.push_render_options(RenderOptions { edge_mode: EdgeMode::Aliased, ..RenderOptions::default() });
        context.draw_rectangle(Some(&solid(Colors::RED)), None, square(12.0, 12.0, 10.0, 10.0), &BoxShadows::default());
        let steel_blue = Color::from_rgb(48, 128, 192);
        context.draw_rectangle(Some(&solid(steel_blue)), None, square(23.0, 12.0, 10.0, 10.0), &BoxShadows::default());
        let orange = Color::from_argb(128, 255, 128, 0);
        context.draw_rectangle(Some(&solid(orange)), None, square(12.0, 23.0, 21.0, 9.0), &BoxShadows::default());
        context.pop_render_options();
    }
    let scale = 2.0;
    let transform = Matrix::create_scale(scale, scale);
    let bounds = Rect::new(0.0, 0.0, f64::from(SIZE) / scale, f64::from(SIZE) / scale);
    for mode in device_modes(&device) {
        for (effect, largest) in &effects {
            let plain = draw_in(&device, mode, |drawing| {
                drawing.set_transform(transform);
                small_content(drawing);
            });
            let mut expected = plain.clone();
            assert!(effect.cpu_reference(&mut expected, usize::from(SIZE), usize::from(SIZE), scale), "{effect}");
            let cpu_layers = crate::drawing_context_impl::cpu_layer_count();
            let actual = draw_in(&device, mode, |drawing| {
                drawing.set_transform(transform);
                assert!(push_custom_effect_graph(drawing, effect, bounds));
                small_content(drawing);
                assert!(pop_custom_effect_graph(drawing, effect, bounds));
            });
            assert_eq!(
                cpu_layers,
                crate::drawing_context_impl::cpu_layer_count(),
                "{mode:?}: {effect} at 2x: a layer was drawn on the processor"
            );
            let (most, mean, _) = difference(&expected, &actual);
            assert!(most <= *largest && mean <= 0.5, "{mode:?}: {effect} at 2x: largest difference {most}, mean {mean}");
            let (changed, _, _) = difference(&plain, &actual);
            assert!(changed > 0, "{mode:?}: {effect} at 2x: the content is changed");
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
/// 0..16, the colour clamped to the alpha (within 3 levels).
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
        // The light is within 2 levels of the exact one (see the lighting
        // test); the content scales the error by at most one.
        assert!(most <= 3 && mean <= 0.5, "{mode:?}: lit content: largest {most}, mean {mean}");
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

// --- runtime shader effects (#455, E5) -----------------------------------------------------

use super::effect_passes::{check_shader_module, shader_failure_reports};
use ferroui_base::media::effects::library::{GridShaderEffect, PixelateEffect, ScanlineShaderEffect, SpotlightShaderEffect};
use ferroui_base::media::effects::{CustomEffect, EffectParameterValue, ImmutableCustomEffect, ShaderEffectDescription};
use ferroui_base::media::imaging::BitmapBlendingMode;

/// The shader effects of the library at values that are not neutral, with
/// the largest difference to their CPU references they are held to (the
/// layer passes are held to 2 levels; the spotlight to 3, as in the tests of
/// `ferroui-skia`).
fn library_shader_effects() -> Vec<(ImmutableCustomEffect, u8)> {
    vec![
        (ImmutableCustomEffect::new(PixelateEffect { cell_size: 6.0 }), 2),
        (ImmutableCustomEffect::new(ScanlineShaderEffect { spacing: 4.0, strength: 0.5 }), 2),
        (
            ImmutableCustomEffect::new(SpotlightShaderEffect {
                center_x: 0.4,
                center_y: 0.5,
                radius: 0.5,
                strength: 0.8,
                color: Colors::ORANGE,
            }),
            3,
        ),
        // The grid (#580), masked by the content as the other overlays.
        (ImmutableCustomEffect::new(GridShaderEffect { cell_size: 12.0, strength: 0.6, ..GridShaderEffect::default() }), 2),
    ]
}

/// The modules of the library's shader effects parse, validate and declare
/// the uniforms of their parameters at the offsets of WGSL's layout; no
/// device is needed.
#[test]
fn the_shader_modules_of_the_library_are_checked_by_naga() {
    for (effect, _) in library_shader_effects() {
        let module = check_shader_module(&effect).unwrap_or_else(|reason| panic!("{effect}: {reason}"));
        assert_eq!(0, module.size() % 16, "{effect}: the block is a multiple of 16 bytes");
        assert_eq!(effect.shader_uniform_layout().len(), module.offsets().len(), "{effect}");
    }
    // The offsets of WGSL's layout: `vec2<f32>` aligned to 8, `vec4<f32>`
    // to 16.
    let layout = |effect: ImmutableCustomEffect| {
        let module = check_shader_module(&effect).unwrap();
        (module.offsets().to_vec(), module.size())
    };
    assert_eq!((vec![0, 8, 12], 16), layout(ImmutableCustomEffect::new(PixelateEffect::default())));
    assert_eq!((vec![0, 8, 12, 16], 32), layout(ImmutableCustomEffect::new(ScanlineShaderEffect::default())));
    assert_eq!(
        (vec![0, 8, 12, 16, 20, 24, 32], 48),
        layout(ImmutableCustomEffect::new(SpotlightShaderEffect::default()))
    );
    assert_eq!((vec![0, 8, 12, 16, 32], 48), layout(ImmutableCustomEffect::new(GridShaderEffect::default())));
}

/// A shader effect of the tests with the parameters `amount` (a number) and
/// `on` (a flag) and the description `$description`.
macro_rules! test_shader_effect {
    ($type:ident, $name:literal, $description:expr) => {
        #[derive(Clone, Debug, Default, PartialEq)]
        struct $type {
            amount: f64,
            on: bool,
        }

        impl CustomEffect for $type {
            const NAME: &'static str = $name;

            fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
                f("amount", self.amount.into());
                f("on", self.on.into());
            }

            fn set_parameter(&mut self, _: &str, _: EffectParameterValue) -> bool {
                false
            }

            fn shader() -> Option<ShaderEffectDescription> {
                Some($description)
            }
        }
    };
}

/// The uniforms of the effects of the tests by the convention.
const TEST_UNIFORMS: &str = "struct Uniforms { layer_size: vec2<f32>, layer_scale: f32, amount: f32, on: i32, }\n";

test_shader_effect!(
    Matching,
    "vello-test-matching-shader",
    ShaderEffectDescription::content_sampling(
        "",
        "struct Uniforms { layer_size: vec2<f32>, layer_scale: f32, amount: f32, on: i32, }\n\
         fn effect(coord: vec2<f32>) -> vec4<f32> { return sample_content(coord) * uniforms.amount * f32(uniforms.on); }"
    )
);

test_shader_effect!(
    DoesNotParse,
    "vello-test-shader-that-does-not-parse",
    ShaderEffectDescription::content_sampling(
        "",
        "struct Uniforms { layer_size: vec2<f32>, layer_scale: f32, amount: f32, on: i32, }\n\
         fn effect(coord: vec2<f32>) -> vec4<f32> { return undefined; }"
    )
);

test_shader_effect!(
    WrongType,
    "vello-test-shader-of-a-wrong-type",
    ShaderEffectDescription::content_sampling(
        "",
        "struct Uniforms { layer_size: vec2<f32>, layer_scale: f32, amount: f32, on: f32, }\n\
         fn effect(coord: vec2<f32>) -> vec4<f32> { return sample_content(coord) * uniforms.amount; }"
    )
);

test_shader_effect!(
    WrongOrder,
    "vello-test-shader-of-a-wrong-order",
    ShaderEffectDescription::content_sampling(
        "",
        "struct Uniforms { layer_scale: f32, layer_size: vec2<f32>, amount: f32, on: i32, }\n\
         fn effect(coord: vec2<f32>) -> vec4<f32> { return sample_content(coord) * uniforms.amount; }"
    )
);

test_shader_effect!(
    Missing,
    "vello-test-shader-with-a-missing-uniform",
    ShaderEffectDescription::content_sampling(
        "",
        "struct Uniforms { layer_size: vec2<f32>, layer_scale: f32, amount: f32, }\n\
         fn effect(coord: vec2<f32>) -> vec4<f32> { return sample_content(coord) * uniforms.amount; }"
    )
);

test_shader_effect!(
    Extra,
    "vello-test-shader-with-an-extra-uniform",
    ShaderEffectDescription::content_sampling(
        "",
        "struct Uniforms { layer_size: vec2<f32>, layer_scale: f32, amount: f32, on: i32, extra: f32, }\n\
         fn effect(coord: vec2<f32>) -> vec4<f32> { return sample_content(coord) * uniforms.amount; }"
    )
);

test_shader_effect!(
    Renamed,
    "vello-test-shader-with-a-renamed-uniform",
    ShaderEffectDescription::content_sampling(
        "",
        "struct Uniforms { layer_size: vec2<f32>, layer_scale: f32, strength: f32, on: i32, }\n\
         fn effect(coord: vec2<f32>) -> vec4<f32> { return sample_content(coord) * uniforms.strength; }"
    )
);

test_shader_effect!(
    OverlaySamplingContent,
    "vello-test-overlay-that-samples-the-content",
    ShaderEffectDescription::overlay(
        "",
        "struct Uniforms { layer_size: vec2<f32>, layer_scale: f32, amount: f32, on: i32, }\n\
         fn effect(coord: vec2<f32>) -> vec4<f32> { return sample_content(coord) * uniforms.amount; }",
        BitmapBlendingMode::SourceOver
    )
);

test_shader_effect!(
    WithChildren,
    "vello-test-shader-with-children",
    ShaderEffectDescription::content_sampling(
        "",
        "struct Uniforms { layer_size: vec2<f32>, layer_scale: f32, amount: f32, on: i32, }\n\
         fn effect(coord: vec2<f32>) -> vec4<f32> { return sample_content(coord) * uniforms.amount; }"
    )
    .with_children(["page"])
);

#[test]
fn a_module_is_checked_against_the_convention() {
    let check = |effect: ImmutableCustomEffect| check_shader_module(&effect).map(|_| ()).map_err(|reason| (effect.name(), reason));
    assert_eq!(Ok(()), check(ImmutableCustomEffect::new(Matching::default())));
    for effect in [
        ImmutableCustomEffect::new(DoesNotParse::default()),
        ImmutableCustomEffect::new(WrongType::default()),
        ImmutableCustomEffect::new(WrongOrder::default()),
        ImmutableCustomEffect::new(Missing::default()),
        ImmutableCustomEffect::new(Extra::default()),
        ImmutableCustomEffect::new(Renamed::default()),
        ImmutableCustomEffect::new(OverlaySamplingContent::default()),
        ImmutableCustomEffect::new(WithChildren::default()),
    ] {
        assert!(check(effect.clone()).is_err(), "{effect}: the module is refused");
    }
}

/// Every blend mode of an overlay composes with WGSL that naga takes.
#[test]
fn the_overlays_of_every_blend_mode_are_valid_wgsl() {
    use wgpu::naga;
    let program = format!("{TEST_UNIFORMS}fn effect(coord: vec2<f32>) -> vec4<f32> {{ return vec4<f32>(0.5, 0.25, 0.0, uniforms.amount); }}");
    for mode in [
        BitmapBlendingMode::Unspecified,
        BitmapBlendingMode::SourceOver,
        BitmapBlendingMode::Source,
        BitmapBlendingMode::Destination,
        BitmapBlendingMode::DestinationOver,
        BitmapBlendingMode::SourceIn,
        BitmapBlendingMode::DestinationIn,
        BitmapBlendingMode::SourceOut,
        BitmapBlendingMode::DestinationOut,
        BitmapBlendingMode::SourceAtop,
        BitmapBlendingMode::DestinationAtop,
        BitmapBlendingMode::Xor,
        BitmapBlendingMode::Plus,
        BitmapBlendingMode::Screen,
        BitmapBlendingMode::Overlay,
        BitmapBlendingMode::Darken,
        BitmapBlendingMode::Lighten,
        BitmapBlendingMode::ColorDodge,
        BitmapBlendingMode::ColorBurn,
        BitmapBlendingMode::HardLight,
        BitmapBlendingMode::SoftLight,
        BitmapBlendingMode::Difference,
        BitmapBlendingMode::Exclusion,
        BitmapBlendingMode::Multiply,
        BitmapBlendingMode::Hue,
        BitmapBlendingMode::Saturation,
        BitmapBlendingMode::Color,
        BitmapBlendingMode::Luminosity,
    ] {
        let source = ShaderEffectDescription::overlay("", program.clone(), mode).wgsl_module();
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|error| panic!("{mode:?}: {}", error.emit_to_string(&source)));
        naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty())
            .validate(&module)
            .unwrap_or_else(|error| panic!("{mode:?}: {}", error.emit_to_string(&source)));
    }
}

/// The premultiplied RGBA pixels of the target with `effect` drawn over the
/// area `area` (whole pixels: x, y, width, height; it may reach beyond the
/// target) by its CPU reference at `scale`, from the content drawn plain
/// (`plain`): nothing of the content outside the area.
fn reference_in_area(plain: &[u8], effect: &ImmutableCustomEffect, area: (i32, i32, i32, i32), scale: f64) -> Vec<u8> {
    let size = i32::from(SIZE);
    let (x0, y0, width, height) = area;
    let at = |x: i32, y: i32| ((y * size + x) * 4) as usize;
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let (tx, ty) = (x0 + x, y0 + y);
            if (0..size).contains(&tx) && (0..size).contains(&ty) {
                let from = at(tx, ty);
                pixels[((y * width + x) * 4) as usize..][..4].copy_from_slice(&plain[from..from + 4]);
            }
        }
    }
    assert!(effect.cpu_reference(&mut pixels, width as usize, height as usize, scale), "{effect}");
    let mut expected = vec![0u8; plain.len()];
    for y in 0..height {
        for x in 0..width {
            let (tx, ty) = (x0 + x, y0 + y);
            if (0..size).contains(&tx) && (0..size).contains(&ty) {
                let to = at(tx, ty);
                expected[to..to + 4].copy_from_slice(&pixels[((y * width + x) * 4) as usize..][..4]);
            }
        }
    }
    expected
}

/// The shader effects of the library drawn by their passes in the hybrid and
/// GPU modes, over the whole target at scales 1 and 2 and over an area that
/// reaches beyond the target, against their CPU references applied to the
/// content drawn plain in the same mode; no layer drawn on the processor.
#[test]
fn the_library_shader_effects_match_their_cpu_references_in_the_gpu_modes() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    for mode in device_modes(&device) {
        for (effect, largest) in library_shader_effects() {
            for scale in [1.0, 2.0] {
                let transform = Matrix::create_scale(scale, scale);
                let plain = draw_in(&device, mode, |drawing| {
                    drawing.set_transform(transform);
                    content(drawing);
                });
                let side = i32::from(SIZE);
                let expected = reference_in_area(&plain, &effect, (0, 0, side, side), scale);
                let cpu_layers = crate::drawing_context_impl::cpu_layer_count();
                let actual = draw_in(&device, mode, |drawing| {
                    drawing.set_transform(transform);
                    let bounds = Rect::new(0.0, 0.0, f64::from(SIZE) / scale, f64::from(SIZE) / scale);
                    drawing.as_drawing_context_impl_with_effects().unwrap().push_effect(Some(bounds), &effect);
                    content(drawing);
                    drawing.as_drawing_context_impl_with_effects().unwrap().pop_effect();
                });
                assert_eq!(
                    cpu_layers,
                    crate::drawing_context_impl::cpu_layer_count(),
                    "{mode:?}: {effect} at {scale}x: drawn on the processor"
                );
                let (most, mean, _) = difference(&expected, &actual);
                assert!(most <= largest && mean <= 0.5, "{mode:?}: {effect} at {scale}x: largest {most}, mean {mean}");
                let (changed, _, _) = difference(&plain, &actual);
                assert!(changed > 0, "{mode:?}: {effect} at {scale}x: the content is changed");
            }

            // An area that begins left of the target: the coordinates of the
            // program are those of the area, its size the whole area.
            let plain = draw_in(&device, mode, |drawing| content(drawing));
            let expected = reference_in_area(&plain, &effect, (-20, 10, 80, 60), 1.0);
            let actual = draw_in(&device, mode, |drawing| {
                let bounds = Rect::new(-20.0, 10.0, 80.0, 60.0);
                drawing.as_drawing_context_impl_with_effects().unwrap().push_effect(Some(bounds), &effect);
                content(drawing);
                drawing.as_drawing_context_impl_with_effects().unwrap().pop_effect();
            });
            let (most, mean, _) = difference(&expected, &actual);
            assert!(most <= largest && mean <= 0.5, "{mode:?}: {effect} beyond the target: largest {most}, mean {mean}");
        }
    }
}

/// A module that does not parse, and one whose uniforms are not those of
/// its parameters, draw their content unchanged in the GPU modes, reported
/// once although drawn in every mode and twice, and nothing is drawn on the
/// processor.
#[test]
fn a_shader_that_fails_draws_unchanged_and_reports_once_in_the_gpu_modes() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    let effects = [
        ImmutableCustomEffect::new(DoesNotParse { amount: 0.5, on: true }),
        ImmutableCustomEffect::new(WrongType { amount: 0.5, on: true }),
    ];
    for mode in device_modes(&device) {
        let plain = draw_in(&device, mode, |drawing| content(drawing));
        for effect in &effects {
            for _ in 0..2 {
                let cpu_layers = crate::drawing_context_impl::cpu_layer_count();
                let actual = draw_in(&device, mode, |drawing| {
                    drawing.as_drawing_context_impl_with_effects().unwrap().push_effect(Some(full()), effect);
                    content(drawing);
                    drawing.as_drawing_context_impl_with_effects().unwrap().pop_effect();
                });
                assert_eq!(cpu_layers, crate::drawing_context_impl::cpu_layer_count(), "{mode:?}: {effect}");
                let (most, _, _) = difference(&plain, &actual);
                assert!(most <= 1, "{mode:?}: {effect}: the content unchanged, largest {most}");
            }
        }
    }
    for effect in &effects {
        assert_eq!(1, shader_failure_reports(effect.name()), "{effect}");
    }
}

/// Frames of the shader effects whose parameters change and whose area moves
/// by a fraction of a pixel take their textures from the pool, and their
/// programs are compiled once: once the first frames filled the pool, no
/// texture is allocated.
#[test]
fn steady_frames_of_the_shader_effects_allocate_no_texture() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    for mode in device_modes(&device) {
        let mut allocations_after_first = None;
        for frame in 0..6 {
            let amount = 0.2 + 0.1 * f64::from(frame);
            let effects = [
                ImmutableCustomEffect::new(PixelateEffect { cell_size: 2.0 + f64::from(frame) }),
                ImmutableCustomEffect::new(ScanlineShaderEffect { spacing: 4.0, strength: amount }),
                ImmutableCustomEffect::new(SpotlightShaderEffect { strength: amount, ..SpotlightShaderEffect::default() }),
            ];
            let bounds = Rect::new(4.0 + 0.25 * f64::from(frame), 6.0, 80.0, 70.0);
            let cpu_layers = crate::drawing_context_impl::cpu_layer_count();
            draw_in(&device, mode, |drawing| {
                for effect in &effects {
                    drawing.as_drawing_context_impl_with_effects().unwrap().push_effect(Some(bounds), effect);
                    content(drawing);
                    drawing.as_drawing_context_impl_with_effects().unwrap().pop_effect();
                }
                // Nested: an overlay over a content-sampling effect.
                drawing.as_drawing_context_impl_with_effects().unwrap().push_effect(Some(bounds), &effects[1]);
                drawing.as_drawing_context_impl_with_effects().unwrap().push_effect(Some(bounds), &effects[0]);
                other_content(drawing);
                drawing.as_drawing_context_impl_with_effects().unwrap().pop_effect();
                drawing.as_drawing_context_impl_with_effects().unwrap().pop_effect();
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

// --- the images of the page transitions (#455, E6) ------------------------------------------

use ferroui_base::media::effects::library::{DissolveTransitionEffect, MaskTransitionEffect};
use ferroui_base::media::effects::EffectImage;
use ferroui_base::platform::{AlphaFormat, PixelFormat};

/// An image of `SIZE` by `SIZE` premultiplied RGBA pixels made by `pixel`:
/// an immutable bitmap of this backend, owned by the handle.
fn effect_image(pixel: impl Fn(u32, u32) -> [u8; 4]) -> EffectImage {
    let side = u32::from(SIZE);
    let mut data = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            data.extend_from_slice(&pixel(x, y));
        }
    }
    let bitmap = crate::immutable_bitmap::ImmutableBitmap::from_pixels(
        PixelSize::new(i32::from(SIZE), i32::from(SIZE)),
        Vector::new(96.0, 96.0),
        i32::from(SIZE) * 4,
        PixelFormat::RGBA8888,
        AlphaFormat::Premul,
        &data,
    )
    .expect("a bitmap");
    EffectImage::from_impl(Arc::new(bitmap))
}

/// The transitions of the library at `progress` over two pages and a
/// pattern of grey levels, and the mask transition with a translucent page.
fn page_transitions(progress: f64) -> Vec<ImmutableCustomEffect> {
    let from = effect_image(|x, y| [(x * 2) as u8, (y * 2) as u8, 200, 255]);
    let to = effect_image(|x, y| [40, (255 - x * 2) as u8, (y * 2) as u8, 255]);
    let pattern = effect_image(|x, y| {
        let level = ((x * 37 + y * 91 + (x * y) % 17) % 256) as u8;
        [level, level, level, 255]
    });
    let glass = effect_image(|x, _| [0, 0, 160, 160 + (x % 2) as u8 * 40]);
    vec![
        ImmutableCustomEffect::new(DissolveTransitionEffect {
            from_page: from,
            to_page: to.clone(),
            pattern: pattern.clone(),
            progress,
            center_x: 0.3,
            center_y: 0.6,
            ..DissolveTransitionEffect::default()
        }),
        ImmutableCustomEffect::new(MaskTransitionEffect { to_page: to, mask: pattern.clone(), progress, softness: 0.2 }),
        ImmutableCustomEffect::new(MaskTransitionEffect { to_page: glass, mask: pattern, progress, softness: 0.5 }),
    ]
}

/// The modules of the transitions bind their images after the content and
/// declare the uniforms of their parameters at the offsets of WGSL's layout;
/// no device is needed.
#[test]
fn the_modules_of_the_page_transitions_are_checked_by_naga() {
    for effect in page_transitions(0.5) {
        check_shader_module(&effect).unwrap_or_else(|reason| panic!("{effect}: {reason}"));
    }
    let layout = |effect: ImmutableCustomEffect| {
        let module = check_shader_module(&effect).unwrap();
        (module.offsets().to_vec(), module.size())
    };
    assert_eq!(
        (vec![0, 8, 12, 16, 20, 32], 48),
        layout(ImmutableCustomEffect::new(DissolveTransitionEffect::default()))
    );
    assert_eq!((vec![0, 8, 12, 16], 32), layout(ImmutableCustomEffect::new(MaskTransitionEffect::default())));
}

/// The transitions drawn by their passes in the hybrid and GPU modes at
/// scales 1 and 2, their images textures of the device, against their CPU
/// references applied to the content drawn plain in the same mode; no layer
/// drawn on the processor.
#[test]
fn the_page_transitions_match_their_cpu_references_in_the_gpu_modes() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    for mode in device_modes(&device) {
        for effect in page_transitions(0.45) {
            for scale in [1.0, 2.0] {
                let transform = Matrix::create_scale(scale, scale);
                let plain = draw_in(&device, mode, |drawing| {
                    drawing.set_transform(transform);
                    content(drawing);
                });
                let side = i32::from(SIZE);
                let expected = reference_in_area(&plain, &effect, (0, 0, side, side), scale);
                let cpu_layers = crate::drawing_context_impl::cpu_layer_count();
                let actual = draw_in(&device, mode, |drawing| {
                    drawing.set_transform(transform);
                    let bounds = Rect::new(0.0, 0.0, f64::from(SIZE) / scale, f64::from(SIZE) / scale);
                    drawing.as_drawing_context_impl_with_effects().unwrap().push_effect(Some(bounds), &effect);
                    content(drawing);
                    drawing.as_drawing_context_impl_with_effects().unwrap().pop_effect();
                });
                assert_eq!(
                    cpu_layers,
                    crate::drawing_context_impl::cpu_layer_count(),
                    "{mode:?}: {effect} at {scale}x: drawn on the processor"
                );
                let (most, mean, _) = difference(&expected, &actual);
                assert!(most <= 3 && mean <= 0.5, "{mode:?}: {effect} at {scale}x: largest {most}, mean {mean}");
                let (changed, _, _) = difference(&plain, &actual);
                assert!(changed > 0, "{mode:?}: {effect} at {scale}x: the content is changed");
            }
        }
    }
}

/// Frames of the transitions whose progress changes upload no image and
/// allocate no texture once the first frames did, and the textures of the
/// images of dropped transitions are released by the next frame; frames
/// whose transitions take new images keep no more textures than they draw.
#[test]
fn the_images_of_the_page_transitions_are_uploaded_once_and_released() {
    use super::effect_passes::{image_texture_count, image_uploads};
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    for mode in device_modes(&device) {
        let before = image_texture_count();
        {
            let effects = page_transitions(0.2);
            let mut counts_after_first = None;
            for frame in 0..6 {
                let progress = 0.2 + 0.1 * f64::from(frame);
                draw_in(&device, mode, |drawing| {
                    for effect in &effects {
                        let value = if let Some(dissolve) = effect.downcast_ref::<DissolveTransitionEffect>() {
                            ImmutableCustomEffect::new(DissolveTransitionEffect { progress, ..dissolve.clone() })
                        } else if let Some(mask) = effect.downcast_ref::<MaskTransitionEffect>() {
                            ImmutableCustomEffect::new(MaskTransitionEffect { progress, ..mask.clone() })
                        } else {
                            effect.clone()
                        };
                        drawing.as_drawing_context_impl_with_effects().unwrap().push_effect(Some(full()), &value);
                        content(drawing);
                        drawing.as_drawing_context_impl_with_effects().unwrap().pop_effect();
                    }
                });
                let counts = (image_uploads(), super::effect_passes::texture_allocations());
                match counts_after_first {
                    None => counts_after_first = Some(counts),
                    Some(first) => assert_eq!(first, counts, "{mode:?}: frame {frame} uploaded or allocated"),
                }
            }
            // Four images: two pages, the pattern and the translucent page.
            assert_eq!(before + 4, image_texture_count(), "{mode:?}");
        }
        // The transitions and their images are dropped: the next frame
        // releases the textures.
        draw_in(&device, mode, |drawing| content(drawing));
        assert_eq!(before, image_texture_count(), "{mode:?}: the textures are released");

        // A new page in every frame.
        for frame in 0..12u32 {
            let page = effect_image(|x, y| [(frame * 20) as u8, x as u8, y as u8, 255]);
            let effect = ImmutableCustomEffect::new(MaskTransitionEffect {
                to_page: page,
                progress: 0.5,
                softness: 1.0,
                ..MaskTransitionEffect::default()
            });
            draw_in(&device, mode, |drawing| {
                drawing.as_drawing_context_impl_with_effects().unwrap().push_effect(Some(full()), &effect);
                content(drawing);
                drawing.as_drawing_context_impl_with_effects().unwrap().pop_effect();
            });
            assert!(image_texture_count() <= before + 1, "{mode:?}: frame {frame}: {} textures", image_texture_count());
        }
        draw_in(&device, mode, |drawing| content(drawing));
        assert_eq!(before, image_texture_count(), "{mode:?}");
    }
}

/// A content-sampling program cannot take images (both backends refuse it):
/// its module is refused, and it draws the content unchanged.
#[test]
fn a_content_sampling_program_with_images_is_refused() {
    #[derive(Clone, Debug, Default, PartialEq)]
    struct SamplingWithImage {
        page: EffectImage,
    }

    impl CustomEffect for SamplingWithImage {
        const NAME: &'static str = "vello-test-sampling-with-an-image";

        fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
            f("page", (&self.page).into());
        }

        fn set_parameter(&mut self, _: &str, _: EffectParameterValue) -> bool {
            false
        }

        fn shader() -> Option<ShaderEffectDescription> {
            Some(
                ShaderEffectDescription::content_sampling(
                    "",
                    "struct Uniforms { layer_size: vec2<f32>, layer_scale: f32, }\n\
                     fn effect(coord: vec2<f32>) -> vec4<f32> { return sample_content(coord) + sample_page(coord); }",
                )
                .with_children(["page"]),
            )
        }
    }

    assert!(check_shader_module(&ImmutableCustomEffect::new(SamplingWithImage::default())).is_err());
}

// --- the interactive effects (#455, E7) ---------------------------------------------------

use super::effect_passes::shader_pipelines_created;
use ferroui_base::media::effects::library::{
    BurningFlameShaderEffect, PointerSpotlightShaderEffect, ReactiveGridShaderEffect, WaterRippleShaderEffect,
};

/// The interactive effects of the library at a frame `t` (0 to 1) of an interaction: the pointer
/// over the area and pressed in odd frames, a ripple aging, the flame burning; with whether each
/// moves the content (which picks the texel under a coordinate it computes: at the edge of a
/// square the last bit of the device's `sin` or `exp` can pick the neighbour).
fn interactive_shader_effects(t: f64) -> Vec<(ImmutableCustomEffect, bool)> {
    let pressed = (t * 10.0).round() as i32 % 2 == 1;
    let mut ripple = WaterRippleShaderEffect {
        pointer_x: 0.3 + 0.4 * t,
        pointer_y: 0.4,
        is_pointer_over: true,
        is_pressed: pressed,
        ..WaterRippleShaderEffect::default()
    };
    ripple.spawn_ripple(Point::new(0.45, 0.55), 1.0);
    ripple.ripples[0].age = 0.2 + 0.3 * t;
    vec![
        (
            ImmutableCustomEffect::new(PointerSpotlightShaderEffect {
                pointer_x: 0.3 + 0.4 * t,
                is_pointer_over: true,
                is_pressed: pressed,
                radius: 0.4,
                ..PointerSpotlightShaderEffect::default()
            }),
            false,
        ),
        (
            ImmutableCustomEffect::new(ReactiveGridShaderEffect {
                pointer_x: 0.6 - 0.2 * t,
                is_pointer_over: true,
                is_pressed: pressed,
                cell_size: 12.0,
                strength: 0.8,
                ..ReactiveGridShaderEffect::default()
            }),
            false,
        ),
        (ImmutableCustomEffect::new(ripple), true),
        (
            ImmutableCustomEffect::new(BurningFlameShaderEffect {
                burn_amount: 0.6 + 0.4 * t,
                flame_phase: 3.0 * t,
                is_pressed: pressed,
                ignition_y: 0.6,
                ..BurningFlameShaderEffect::default()
            }),
            true,
        ),
    ]
}

/// The modules of the interactive effects parse, validate and declare the uniforms of their
/// parameters at the offsets of WGSL's layout; no device is needed.
#[test]
fn the_shader_modules_of_the_interactive_effects_are_checked_by_naga() {
    for (effect, _) in interactive_shader_effects(0.5) {
        let module = check_shader_module(&effect).unwrap_or_else(|reason| panic!("{effect}: {reason}"));
        assert_eq!(0, module.size() % 16, "{effect}: the block is a multiple of 16 bytes");
        assert_eq!(effect.shader_uniform_layout().len(), module.offsets().len(), "{effect}");
    }
    // The flags are `i32`, the colour a `vec4<f32>` aligned to 16.
    let module = check_shader_module(&ImmutableCustomEffect::new(PointerSpotlightShaderEffect::default())).unwrap();
    assert_eq!((vec![0, 8, 12, 16, 20, 24, 28, 32, 36, 48], 64), (module.offsets().to_vec(), module.size()));
}

/// The interactive effects drawn by their passes in the hybrid and GPU modes at scales 1 and 2,
/// against their CPU references applied to the content drawn plain in the same mode: the overlays
/// within 3 levels, the programs that move the content within a mean of a level and a half and at
/// most 1% of the pixels further than 2 levels; no layer drawn on the processor.
#[test]
fn the_interactive_effects_match_their_cpu_references_in_the_gpu_modes() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    let pixels = usize::from(SIZE) * usize::from(SIZE);
    for mode in device_modes(&device) {
        for t in [0.0, 0.5, 1.0] {
            for (effect, moves) in interactive_shader_effects(t) {
                for scale in [1.0, 2.0] {
                    let transform = Matrix::create_scale(scale, scale);
                    let plain = draw_in(&device, mode, |drawing| {
                        drawing.set_transform(transform);
                        content(drawing);
                    });
                    let side = i32::from(SIZE);
                    let expected = reference_in_area(&plain, &effect, (0, 0, side, side), scale);
                    let cpu_layers = crate::drawing_context_impl::cpu_layer_count();
                    let actual = draw_in(&device, mode, |drawing| {
                        drawing.set_transform(transform);
                        let bounds = Rect::new(0.0, 0.0, f64::from(SIZE) / scale, f64::from(SIZE) / scale);
                        drawing.as_drawing_context_impl_with_effects().unwrap().push_effect(Some(bounds), &effect);
                        content(drawing);
                        drawing.as_drawing_context_impl_with_effects().unwrap().pop_effect();
                    });
                    assert_eq!(
                        cpu_layers,
                        crate::drawing_context_impl::cpu_layer_count(),
                        "{mode:?}: {effect} at {scale}x: drawn on the processor"
                    );
                    let (most, mean, far) = difference(&expected, &actual);
                    if moves {
                        assert!(
                            far * 100 <= pixels && mean <= 1.5,
                            "{mode:?}: {effect} at {scale}x: {far} pixels further than 2 levels, mean {mean}"
                        );
                    } else {
                        assert!(most <= 3 && mean <= 0.5, "{mode:?}: {effect} at {scale}x: largest {most}, mean {mean}");
                    }
                    let (changed, _, _) = difference(&plain, &actual);
                    assert!(changed > 0, "{mode:?}: {effect} at {scale}x: the content is changed");
                }
            }
        }
    }
}

/// Frames of an interaction change only the uniforms of the interactive effects: once the first
/// frames compiled their pipelines and filled the pool, no frame compiles a pipeline or allocates
/// a texture, and none is drawn on the processor.
#[test]
fn frames_of_the_interactive_effects_compile_their_pipelines_once() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    for mode in device_modes(&device) {
        let mut after_first = None;
        for frame in 0..8 {
            let effects = interactive_shader_effects(f64::from(frame) / 7.0);
            let cpu_layers = crate::drawing_context_impl::cpu_layer_count();
            draw_in(&device, mode, |drawing| {
                for (effect, _) in &effects {
                    drawing.as_drawing_context_impl_with_effects().unwrap().push_effect(Some(full()), effect);
                    content(drawing);
                    drawing.as_drawing_context_impl_with_effects().unwrap().pop_effect();
                }
            });
            assert_eq!(cpu_layers, crate::drawing_context_impl::cpu_layer_count(), "{mode:?}: drawn on the processor");
            let counts = (shader_pipelines_created(), super::effect_passes::texture_allocations());
            if frame == 1 {
                after_first = Some(counts);
            } else if let Some(previous) = after_first {
                assert_eq!(previous, counts, "{mode:?}: frame {frame} compiled a pipeline or allocated a texture");
            }
        }
    }
    for (effect, _) in interactive_shader_effects(0.0) {
        assert_eq!(0, shader_failure_reports(effect.name()), "{effect}");
    }
}

// --- the whole library (#455, E9) ----------------------------------------------------------

/// Every effect of the library at a frame `t` (0 to 1) of an animation, the
/// transitions over the images of `transitions` (made once, so that a frame
/// changes only their progress): the graph effects (colour matrices, the glow,
/// the convolutions), the runtime shaders, the page transitions and the
/// interactive effects, each parameter that animates moving with `t`. The
/// glow keeps its blur radius (its padding, and so the size of its layer,
/// follow it) and animates its intensity.
fn whole_library(t: f64, transitions: &[ImmutableCustomEffect]) -> Vec<ImmutableCustomEffect> {
    use ferroui_base::media::effects::library::{
        BrightnessContrastEffect, EdgeDetectEffect, GlowEffect, GrayscaleEffect, InvertEffect, SaturationEffect,
        SepiaEffect, SharpenEffect, TintEffect,
    };
    let mut effects = vec![
        ImmutableCustomEffect::new(TintEffect { color: Colors::TEAL, strength: 0.2 + 0.6 * t }),
        ImmutableCustomEffect::new(GrayscaleEffect { amount: 0.2 + 0.8 * t }),
        ImmutableCustomEffect::new(SepiaEffect { amount: 0.1 + 0.9 * t }),
        ImmutableCustomEffect::new(SaturationEffect { saturation: 0.5 + 1.5 * t }),
        ImmutableCustomEffect::new(BrightnessContrastEffect { brightness: 0.2 * t, contrast: 0.8 + 0.6 * t }),
        ImmutableCustomEffect::new(InvertEffect { amount: t }),
        ImmutableCustomEffect::new(GlowEffect { color: Colors::BLUE, blur_radius: 4.0, intensity: 0.3 + 0.6 * t }),
        ImmutableCustomEffect::new(SharpenEffect { strength: 0.2 + 0.8 * t }),
        ImmutableCustomEffect::new(EdgeDetectEffect { strength: 0.5 + 0.5 * t }),
        ImmutableCustomEffect::new(PixelateEffect { cell_size: 2.0 + 6.0 * t }),
        ImmutableCustomEffect::new(ScanlineShaderEffect { spacing: 4.0, strength: 0.2 + 0.6 * t }),
        ImmutableCustomEffect::new(SpotlightShaderEffect { strength: 0.2 + 0.6 * t, ..SpotlightShaderEffect::default() }),
    ];
    for effect in transitions {
        effects.push(if let Some(dissolve) = effect.downcast_ref::<DissolveTransitionEffect>() {
            ImmutableCustomEffect::new(DissolveTransitionEffect { progress: t, ..dissolve.clone() })
        } else if let Some(mask) = effect.downcast_ref::<MaskTransitionEffect>() {
            ImmutableCustomEffect::new(MaskTransitionEffect { progress: t, ..mask.clone() })
        } else {
            effect.clone()
        });
    }
    effects.extend(interactive_shader_effects(t).into_iter().map(|(effect, _)| effect));
    effects
}

/// Draws `effect` around the content over the whole target as the render
/// walk draws it: a graph effect as its graph, any other as an effect of the
/// drawing context.
fn draw_library_effect(drawing: &mut DrawingContextImpl, effect: &ImmutableCustomEffect) {
    use ferroui_base::rendering::composition::effects::{pop_custom_effect_graph, push_custom_effect_graph};
    if effect.has_graph() {
        assert!(push_custom_effect_graph(drawing, effect, full()), "{effect}");
        content(drawing);
        assert!(pop_custom_effect_graph(drawing, effect, full()), "{effect}");
    } else {
        drawing.as_drawing_context_impl_with_effects().unwrap().push_effect(Some(full()), effect);
        content(drawing);
        drawing.as_drawing_context_impl_with_effects().unwrap().pop_effect();
    }
}

/// The steady state of the whole library in the hybrid and GPU modes: frames
/// that draw every effect of the library with parameters that change compile
/// no pipeline, allocate no texture and upload no image once the first frames
/// compiled the pipelines, filled the pool and uploaded the images; nothing
/// is drawn on the processor and no effect fails. Effect graphs and programs
/// are compiled once, uniforms are written per frame.
#[test]
fn frames_of_the_whole_library_compile_and_allocate_nothing_in_the_gpu_modes() {
    use super::effect_passes::image_uploads;
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    let transitions = page_transitions(0.0);
    for mode in device_modes(&device) {
        let mut after_first = None;
        for frame in 0..8 {
            let effects = whole_library(f64::from(frame) / 7.0, &transitions);
            let cpu_layers = crate::drawing_context_impl::cpu_layer_count();
            draw_in(&device, mode, |drawing| {
                for effect in &effects {
                    draw_library_effect(drawing, effect);
                }
            });
            assert_eq!(cpu_layers, crate::drawing_context_impl::cpu_layer_count(), "{mode:?}: drawn on the processor");
            let counts = (shader_pipelines_created(), super::effect_passes::texture_allocations(), image_uploads());
            if frame == 2 {
                after_first = Some(counts);
            } else if let Some(previous) = after_first {
                assert_eq!(
                    previous, counts,
                    "{mode:?}: frame {frame} compiled a pipeline, allocated a texture or uploaded an image"
                );
            }
        }
    }
    for effect in whole_library(0.0, &transitions) {
        assert_eq!(0, shader_failure_reports(effect.name()), "{effect}");
    }
}

/// What stands for the prewarm on Vello: its render targets do not prewarm
/// (gpu-rendering.md, "Pipelines made ahead of their first draw": vello and
/// vello_hybrid make their own pipelines when their renderers are made), and
/// the pipeline of a shader effect type is made by the first frame that draws
/// the type, once per device. The first frame of the library makes one
/// pipeline per runtime shader of the library, and no frame after it, in
/// this mode or the other, makes one.
#[test]
fn the_first_frame_of_the_library_makes_one_pipeline_per_shader_type() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    let transitions = page_transitions(0.0);
    let shader_types = whole_library(0.0, &transitions)
        .iter()
        .filter(|effect| effect.has_shader())
        .map(|effect| effect.value_type_id())
        .collect::<std::collections::HashSet<_>>()
        .len() as u64;
    assert_eq!(9, shader_types, "the runtime shaders of the library");
    let before = shader_pipelines_created();
    for mode in device_modes(&device) {
        for frame in 0..3 {
            let effects = whole_library(0.2 + 0.3 * f64::from(frame), &transitions);
            draw_in(&device, mode, |drawing| {
                for effect in &effects {
                    draw_library_effect(drawing, effect);
                }
            });
            // The first frame of the first mode made them all; no frame
            // after it made one.
            let made = shader_pipelines_created() - before;
            assert_eq!(shader_types, made, "{mode:?}: frame {frame}: {made} pipelines made, one per shader type");
        }
    }
}
