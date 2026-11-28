//! What reaches the CPU in the device modes (#1146): each effect of the drawing context is drawn
//! once, and counts as on the CPU if it made a scene of the CPU renderer, computed a layer on the
//! processor, or refused the layer, which leaves it to the pixels of the compositor.
//!
//! Skipped with a message on a machine without a graphics adapter.

use crate::drawing_context_impl::{cpu_layer_count, CreateInfo, DrawingContextImpl};
use crate::gpu::VelloWgpuDevice;
use crate::scene::{cpu_scenes_made, IVelloSceneSink};
use crate::vello_options::VelloRenderingMode;
use ferroui_base::media::effects::library::{GrayscaleEffect, PixelateEffect};
use ferroui_base::media::effects::{ImmutableBlurEffect, ImmutableCustomEffect, ImmutableDropShadowEffect};
use ferroui_base::media::imaging::BitmapBlendingMode;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{
    BoxShadows, ColorMatrix, ColorMatrixAlphaMode, Colors, ConvolveMatrix, ImmutableLuminanceToAlphaEffect,
};
use ferroui_base::platform::{
    IDrawingContextImpl, LightingParameters, MultiplyLayerFactors, PixelOperation,
};
use ferroui_base::rendering::composition::effects::{pop_custom_effect_graph, push_custom_effect_graph};
use ferroui_base::rendering::composition::server::{LightColor, LightMap, LightSource, LightingMaterial};
use ferroui_base::{Matrix, PixelSize, Rect, RoundedRect, Vector};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

const SIZE: u16 = 64;
const BOUNDS: Rect = Rect::new(8.0, 8.0, 48.0, 48.0);
const MATERIAL: LightingMaterial = LightingMaterial { ambient: 0.5, diffuse: 0.7, specular: 0.6, shine: 24.0 };

fn content(context: &mut dyn IDrawingContextImpl) {
    let brush = ImmutableSolidColorBrush::new(Colors::RED);
    context.draw_rectangle(Some(&brush), None, RoundedRect::from_rect(Rect::new(16.0, 16.0, 32.0, 32.0)), &BoxShadows::default());
}

fn lights() -> LightMap {
    let mut map = LightMap::new();
    map.add(LightSource::Ambient { color: LightColor::new(0.5, 0.5, 0.5) }, Matrix::IDENTITY);
    map
}

/// Draws one effect. `false` when the context refused it.
type Case = fn(&mut DrawingContextImpl) -> bool;

fn through_effect(context: &mut DrawingContextImpl, effect: &dyn ferroui_base::media::effects::IEffect) -> bool {
    let effects = context.as_drawing_context_impl_with_effects().unwrap();
    effects.push_effect(Some(BOUNDS), effect);
    content(effects);
    effects.pop_effect();
    true
}

fn cases() -> Vec<(&'static str, Case)> {
    vec![
        ("blur effect", |c| through_effect(c, &ImmutableBlurEffect::new(6.0))),
        ("drop shadow effect", |c| through_effect(c, &ImmutableDropShadowEffect::new(3.0, 3.0, 4.0, Colors::BLUE, 0.8))),
        ("luminance to alpha", |c| through_effect(c, &ImmutableLuminanceToAlphaEffect)),
        ("shader effect", |c| through_effect(c, &ImmutableCustomEffect::new(PixelateEffect { cell_size: 6.0 }))),
        ("effect graph", |c| {
            let effect = ImmutableCustomEffect::new(GrayscaleEffect { amount: 0.8 });
            if !push_custom_effect_graph(c, &effect, BOUNDS) {
                return false;
            }
            content(c);
            pop_custom_effect_graph(c, &effect, BOUNDS)
        }),
        ("blend layer", |c| {
            let layers = c.as_drawing_context_impl_with_layer_effects().unwrap();
            layers.push_blend_layer(BOUNDS, BitmapBlendingMode::Multiply);
            content(layers);
            layers.pop_blend_layer();
            true
        }),
        ("colour matrix layer", |c| {
            let layers = c.as_drawing_context_impl_with_layer_effects().unwrap();
            layers.push_color_matrix_layer(BOUNDS, &ColorMatrix::invert(), ColorMatrixAlphaMode::Premultiplied);
            content(layers);
            layers.pop_color_matrix_layer();
            true
        }),
        ("multiply layer of factors", |c| {
            let factors = MultiplyLayerFactors {
                size: PixelSize::new(1, 1),
                rgba: Arc::new(vec![128, 128, 128, 255]),
                scale: 1.0,
            };
            let layers = c.as_drawing_context_impl_with_layer_effects().unwrap();
            layers.push_multiply_layer(BOUNDS, &factors);
            content(layers);
            layers.pop_multiply_layer();
            true
        }),
        ("convolution layer", |c| {
            let matrix = ConvolveMatrix::new(3, 3, &[0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0]).unwrap();
            let layers = c.as_drawing_context_impl_with_layer_effects().unwrap();
            layers.push_convolve_matrix_layer(BOUNDS, &matrix);
            content(layers);
            layers.pop_convolve_matrix_layer();
            true
        }),
        ("lit layer", |c| {
            let map = lights();
            let lighting = LightingParameters::new(&map, MATERIAL);
            let layers = c.as_drawing_context_impl_with_layer_effects().unwrap();
            if !layers.push_lighting_multiply_layer(BOUNDS, &lighting) {
                return false;
            }
            content(layers);
            layers.pop_multiply_layer();
            true
        }),
        ("lighting", |c| {
            let map = lights();
            let lighting = LightingParameters::new(&map, MATERIAL);
            c.as_drawing_context_impl_with_layer_effects().unwrap().draw_lighting(BOUNDS, &lighting)
        }),
        ("pixel operation", |c| {
            let layers = c.as_drawing_context_impl_with_layer_effects().unwrap();
            if !layers.push_pixel_operation(BOUNDS, &PixelOperation::Arithmetic { k: [0.5, 0.25, 0.5, 0.05] }) {
                return false;
            }
            content(layers);
            layers.next_pixel_operation_source();
            content(layers);
            layers.pop_pixel_operation();
            true
        }),
        ("opacity mask", |c| {
            let mask = ImmutableSolidColorBrush::new(Colors::BLACK);
            c.push_opacity_mask(&mask, BOUNDS);
            content(c);
            c.pop_opacity_mask();
            true
        }),
    ]
}

fn drawing(device: &Arc<VelloWgpuDevice>, mode: VelloRenderingMode, pixels: &Rc<RefCell<Vec<u8>>>) -> DrawingContextImpl {
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

/// What still reaches the CPU in a device mode: the guard of #1146. An effect that is given a
/// device route leaves this list, and one that starts to reach the CPU fails the test.
fn known_on_the_cpu() -> &'static [&'static str] {
    // A multiply layer of factors takes an image of the processor in every mode.
    &["multiply layer of factors"]
}

#[test]
fn only_the_known_effects_reach_the_cpu_in_the_device_modes() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    let mut modes = Vec::new();
    if cfg!(feature = "hybrid") {
        modes.push(VelloRenderingMode::Hybrid);
    }
    if cfg!(feature = "gpu") && device.supports_compute() {
        modes.push(VelloRenderingMode::Gpu);
    }
    for mode in modes {
        let mut on_the_cpu = Vec::new();
        for (name, case) in cases() {
            let pixels = Rc::new(RefCell::new(vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4]));
            let mut context = drawing(&device, mode, &pixels);
            let (scenes, layers) = (cpu_scenes_made(), cpu_layer_count());
            let accepted = case(&mut context);
            context.dispose();
            // A layer the context refuses is left to the pixels of the compositor.
            if !accepted || cpu_scenes_made() != scenes || cpu_layer_count() != layers {
                on_the_cpu.push(name);
            }
        }
        assert_eq!(on_the_cpu, known_on_the_cpu(), "{mode:?}");
    }
}
