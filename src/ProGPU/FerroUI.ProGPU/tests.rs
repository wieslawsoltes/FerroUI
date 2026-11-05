//! The engine on the device of the backend, headless.

use crate::ProGpuDevice;
use progpu_native_sys::progpu::*;
use progpu_native_sys::webgpu::*;
use std::ptr;

/// A device, or `None` (with a message) where the runtime or a GPU is not available.
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
    let w = &device.runtime().wgpu;
    let p = &device.runtime().progpu;
    let (width, height) = (40u32, 30u32);
    // SAFETY: the calls of the C ABIs with the handles of the device.
    unsafe {
        let texture = w.wgpuDeviceCreateTexture(
            device.device(),
            &WGPUTextureDescriptor {
                nextInChain: ptr::null(),
                label: ptr::null(),
                usage: (WGPUTextureUsage_RenderAttachment | WGPUTextureUsage_CopySrc) as u32,
                dimension: WGPUTextureDimension_2D,
                size: WGPUExtent3D { width, height, depthOrArrayLayers: 1 },
                format: WGPUTextureFormat_RGBA8Unorm,
                mipLevelCount: 1,
                sampleCount: 1,
                viewFormatCount: 0,
                viewFormats: ptr::null(),
            },
        );
        let view = w.wgpuTextureCreateView(texture, ptr::null());
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
        let status = p.progpu_native_engine_render(device.engine(), &frame, &mut metrics);
        assert_eq!(PROGPU_NATIVE_STATUS_SUCCESS, status, "{}", device.last_error());
        let pixels = device.read_rgba8(texture, width, height);
        let pixel = |x: u32, y: u32| &pixels[((y * width + x) * 4) as usize..][..4];
        assert_eq!([0, 0, 0, 255], pixel(2, 2));
        assert_eq!([0, 255, 0, 255], pixel(20, 15));
        w.wgpuTextureViewRelease(view);
        w.wgpuTextureRelease(texture);
    }
}
