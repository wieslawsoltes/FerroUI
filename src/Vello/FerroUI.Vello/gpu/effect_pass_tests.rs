//! The render passes of `effect_passes.rs` in the GPU modes: the lighting
//! of the compositor drawn by the device, compared with the exact light (the
//! light of the compositor in double precision at the point each pixel
//! shows, rounded to 8 bits), and the frames of a scroll allocating nothing
//! once the pool holds the textures they need.
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
