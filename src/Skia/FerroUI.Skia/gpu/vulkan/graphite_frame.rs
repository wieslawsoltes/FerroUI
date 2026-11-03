//! Vulkan texture and final-layout operations missing in skia-safe 0.153.
//! All C++ types stay opaque on this side of the bridge.
use ferroui_vulkan::VulkanImageInfo;
use skia_safe::gpu::graphite::{BackendTexture, Context, Recording};
use skia_safe::wrapper::{PointerWrapper, ValueWrapper};
use skia_safe::Surface;
use std::ffi::c_void;

unsafe extern "C" {
    fn ferro_graphite_wrap(
        texture: *mut c_void,
        width: i32,
        height: i32,
        image: u64,
        memory: u64,
        bytes: u64,
        format: u32,
        usage: u32,
        tiling: u32,
        layout: u32,
        queue: u32,
        levels: u32,
        samples: u32,
        protected_image: bool,
    );
    fn ferro_graphite_submit(
        context: *mut c_void,
        recording: *mut c_void,
        surface: *mut c_void,
        layout: u32,
        queue: u32,
    ) -> bool;
}

pub(super) fn wrap(info: &VulkanImageInfo, queue: u32) -> BackendTexture {
    let mut texture = BackendTexture::default();
    // SAFETY: the session owns the image and its memory until presentation.
    // ValueWrapper exposes the initialized BackendTexture for C++ assignment.
    unsafe {
        ferro_graphite_wrap(
            std::ptr::from_mut(ValueWrapper::inner_mut(&mut texture)).cast(),
            info.pixel_size.width,
            info.pixel_size.height,
            info.handle,
            info.memory_handle,
            info.memory_size,
            info.format,
            info.usage_flags,
            info.tiling,
            info.layout,
            queue,
            info.level_count,
            info.sample_count,
            info.is_protected,
        );
    }
    texture
}

pub(in crate::gpu) fn submit(
    context: &mut Context,
    recording: &mut Recording,
    surface: &mut Surface,
    layout: u32,
    queue: u32,
) {
    // SAFETY: these three live Skia objects belong to this frame. The bridge
    // borrows them only while inserting the recording and submitting its commands.
    let ok = unsafe {
        ferro_graphite_submit(
            std::ptr::from_mut(PointerWrapper::inner_mut(context)).cast(),
            std::ptr::from_mut(PointerWrapper::inner_mut(recording)).cast(),
            std::ptr::from_mut(PointerWrapper::inner_mut(surface)).cast(),
            layout,
            queue,
        )
    };
    assert!(ok, "Graphite failed to submit the Vulkan frame");
}
