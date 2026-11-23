//! The route of a box shadow in the GPU modes (#1139): no shadow reaches a scene of the CPU
//! renderer, whatever its shape. The shapes are the ones with a closed form and the ones without:
//! a box too small for its blur, corners that are not circular, and an inset shadow.
//!
//! Skipped with a message on a machine without a graphics adapter.

use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::gpu::VelloWgpuDevice;
use crate::scene::{cpu_scenes_made, IVelloSceneSink};
use crate::vello_options::VelloRenderingMode;
use ferroui_base::media::{BoxShadow, BoxShadows, Color};
use ferroui_base::platform::IDrawingContextImpl;
use ferroui_base::{Rect, RoundedRect, Vector};
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

fn shadow(blur: f64, is_inset: bool) -> BoxShadows {
    BoxShadows::new(BoxShadow {
        offset_x: 2.0,
        offset_y: 3.0,
        blur,
        color: Color::from_argb(200, 0, 0, 0),
        is_inset,
        ..BoxShadow::default()
    })
}

/// A name, a box and its shadows.
fn shapes() -> Vec<(&'static str, RoundedRect, BoxShadows)> {
    let corner = Vector::new(16.0, 8.0);
    vec![
        ("closed form", RoundedRect::from_radius(Rect::new(24.0, 24.0, 48.0, 48.0), 6.0), shadow(12.0, false)),
        ("too small for its blur", RoundedRect::from_radius(Rect::new(20.0, 40.0, 56.0, 14.0), 7.0), shadow(20.0, false)),
        (
            "elliptical corners",
            RoundedRect::new(Rect::new(20.0, 24.0, 56.0, 48.0), corner, corner, corner, corner),
            shadow(12.0, false),
        ),
        ("inset", RoundedRect::from_radius(Rect::new(16.0, 16.0, 64.0, 64.0), 8.0), shadow(12.0, true)),
    ]
}

fn draw(device: &Arc<VelloWgpuDevice>, mode: VelloRenderingMode, shape: RoundedRect, shadows: &BoxShadows) -> Vec<u8> {
    let sink: Box<dyn IVelloSceneSink> =
        crate::gpu::create_window_scene_sink(device, &[mode], SIZE, SIZE, wgpu::TextureFormat::Rgba8Unorm);
    let pixels = Rc::new(RefCell::new(vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4]));
    let target = pixels.clone();
    let mut drawing = DrawingContextImpl::new(
        CreateInfo {
            sink,
            backdrop: None,
            on_finished: Box::new(move |sink| sink.render_to_pixels(&mut target.borrow_mut())),
            scale_drawing_to_dpi: false,
            dpi: Vector::new(96.0, 96.0),
            rendering_modes: vec![mode],
        },
        Vec::new(),
    );
    drawing.draw_rectangle(None, None, shape, shadows);
    drawing.dispose();
    pixels.take()
}

#[test]
fn no_shadow_reaches_a_scene_of_the_cpu_renderer_in_the_device_modes() {
    let device = match VelloWgpuDevice::headless() {
        Ok(device) => device,
        Err(error) => {
            println!("skipped: no adapter: {error}");
            return;
        }
    };
    for mode in device_modes(&device) {
        for (name, shape, shadows) in shapes() {
            let before = cpu_scenes_made();
            let pixels = draw(&device, mode, shape, &shadows);
            assert_eq!(before, cpu_scenes_made(), "{mode:?}, {name}: a CPU scene was made");
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0), "{mode:?}, {name}: nothing was drawn");
        }
    }
}
