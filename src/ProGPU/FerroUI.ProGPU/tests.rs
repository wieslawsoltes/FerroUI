//! The engine on the device of the backend, headless.

use crate::ProGpuDevice;
use progpu_native_sys::progpu::*;
use progpu_native_sys::webgpu::*;
use std::ptr;

/// A device, or `None` (with a message) where no GPU is available.
fn device() -> Option<ProGpuDevice> {
    match ProGpuDevice::new() {
        Ok(device) => Some(device),
        Err(error) => {
            eprintln!("skipped: {error}");
            None
        }
    }
}

#[test]
fn the_engine_clears_a_target_and_draws_rectangles() {
    let Some(device) = device() else { return };
    let (width, height) = (40u32, 30u32);
    // SAFETY: the calls of the C ABIs with the handles of the device.
    unsafe {
        let (texture, view) = device.create_target(width, height);
        let rect = progpu_native_rect {
            x: 10.0,
            y: 5.0,
            width: 20.0,
            height: 20.0,
            color: progpu_native_color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 },
        };
        let frame = progpu_native_frame {
            struct_size: std::mem::size_of::<progpu_native_frame>() as u32,
            width,
            height,
            dpi_scale: 1.0,
            target_view: view as usize,
            clear_color: progpu_native_color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
            rects: &rect,
            rect_count: 1,
            draw_state: ptr::null(),
        };
        let mut metrics: progpu_native_frame_metrics = std::mem::zeroed();
        metrics.struct_size = std::mem::size_of::<progpu_native_frame_metrics>() as u32;
        let status = progpu_native_engine_render(device.engine(), &frame, &mut metrics);
        assert_eq!(PROGPU_NATIVE_STATUS_SUCCESS, status, "{}", device.last_error());
        let pixels = device.read_rgba8(texture, width, height);
        let pixel = |x: u32, y: u32| &pixels[((y * width + x) * 4) as usize..][..4];
        assert_eq!([0, 0, 0, 255], pixel(2, 2));
        assert_eq!([0, 255, 0, 255], pixel(20, 15));
        wgpuTextureViewRelease(view);
        wgpuTextureRelease(texture);
    }
}

#[test]
fn a_retained_scene_renders_and_a_stable_frame_uploads_nothing() {
    use crate::scene::{SceneBuilder, NO_INDEX};
    let Some(device) = device() else { return };
    let (width, height) = (32u32, 32u32);
    let mut builder = SceneBuilder::new();
    builder.clear_target(progpu_native_color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 }).unwrap();
    let brush = builder.add_solid_brush(progpu_native_color { r: 1.0, g: 1.0, b: 0.0, a: 1.0 }, 1.0).unwrap();
    let primitive = progpu_native_analytic_primitive {
        kind: PROGPU_NATIVE_PRIMITIVE_ELLIPSE as u32,
        flags: 0,
        x: 4.0,
        y: 4.0,
        width: 24.0,
        height: 24.0,
        corner_radius: 0.0,
        stroke_thickness: 0.0,
        color: progpu_native_color { r: 1.0, g: 1.0, b: 0.0, a: 1.0 },
        transform: SceneBuilder::identity_transform(),
    };
    let bounds = progpu_native_image_rect { x: 4.0, y: 4.0, width: 24.0, height: 24.0 };
    builder.draw_analytic(&[primitive], &[brush], bounds, NO_INDEX).unwrap();
    let (scene_id, generation) = (builder.scene_id(), builder.generation());
    device.update_scene(builder.build().unwrap()).unwrap();
    let (texture, view) = device.create_target(width, height);
    device.render_scene(scene_id, generation, view, width, height, 1.0).unwrap();
    let stable = device.render_scene(scene_id, generation, view, width, height, 1.0).unwrap();
    assert_eq!(0, stable.vertex_upload_bytes + stable.index_upload_bytes + stable.texture_upload_bytes);
    let pixels = device.read_rgba8(texture, width, height);
    let pixel = |x: u32, y: u32| pixels[((y * width + x) * 4) as usize..][..4].to_vec();
    assert_eq!(vec![0, 0, 255, 255], pixel(1, 1), "the cleared target outside the ellipse");
    assert_eq!(vec![255, 255, 0, 255], pixel(16, 16), "the center of the ellipse");
    // SAFETY: the handles are owned by the test.
    unsafe {
        wgpuTextureViewRelease(view);
        wgpuTextureRelease(texture);
    }
}
