//! Rendering with Vulkan through Skia's configured GPU backend.
//!
//! The GPU over the graphics context of a Vulkan device
//! ([`VulkanSkiaGpu`]), the render target that draws a frame into the image
//! of a Vulkan render session ([`VulkanSkiaRenderTarget`]) and the import of
//! images of other devices ([`VulkanSkiaExternalObjectsFeature`]).
//!
//! The module needs a Skia build with Vulkan (the `ferro_skia_vulkan`
//! configuration: Linux, and the feature `vulkan` of this crate elsewhere).

#[cfg(ferro_skia_graphite_vulkan)]
pub(super) mod graphite_frame;

#[cfg(ferro_skia_graphite_vulkan)]
use super::graphite::GraphiteGrContext as VulkanGrContext;
#[cfg(ferro_skia_vulkan)]
use super::ganesh::GaneshGrContext as VulkanGrContext;

#[cfg(ferro_skia_vulkan)]
mod vulkan_skia_external_objects_feature;
mod vulkan_skia_gpu;
mod vulkan_skia_render_target;

#[cfg(ferro_skia_vulkan)]
pub use vulkan_skia_external_objects_feature::VulkanSkiaExternalObjectsFeature;
pub use vulkan_skia_gpu::VulkanSkiaGpu;
pub use vulkan_skia_render_target::VulkanSkiaRenderTarget;

#[cfg(ferro_skia_vulkan)]
use ferroui_vulkan::VulkanImageInfo;
#[cfg(ferro_skia_vulkan)]
use skia_safe::gpu::vk;
#[cfg(ferro_skia_vulkan)]
use skia_safe::gpu::Protected;

/// The format of an image as Skia names it. The framework renders to the
/// two formats with 8 bits per channel and no others.
#[cfg(ferro_skia_vulkan)]
pub(crate) fn sk_format(format: u32) -> Option<vk::Format> {
    use ferroui_vulkan::ash::vk::Format;
    if format == Format::R8G8B8A8_UNORM.as_raw() as u32 {
        Some(vk::Format::R8G8B8A8_UNORM)
    } else if format == Format::B8G8R8A8_UNORM.as_raw() as u32 {
        Some(vk::Format::B8G8R8A8_UNORM)
    } else {
        None
    }
}

/// The layout of an image as Skia names it, for the layouts of the first
/// version of Vulkan and the one of a presented image.
#[cfg(ferro_skia_vulkan)]
pub(crate) fn sk_image_layout(layout: u32) -> Option<vk::ImageLayout> {
    use ferroui_vulkan::ash::vk::ImageLayout;
    let layout = ImageLayout::from_raw(layout as i32);
    Some(match layout {
        ImageLayout::UNDEFINED => vk::ImageLayout::UNDEFINED,
        ImageLayout::GENERAL => vk::ImageLayout::GENERAL,
        ImageLayout::COLOR_ATTACHMENT_OPTIMAL => vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL => vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
        ImageLayout::SHADER_READ_ONLY_OPTIMAL => vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        ImageLayout::TRANSFER_SRC_OPTIMAL => vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        ImageLayout::TRANSFER_DST_OPTIMAL => vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        ImageLayout::PREINITIALIZED => vk::ImageLayout::PREINITIALIZED,
        ImageLayout::PRESENT_SRC_KHR => vk::ImageLayout::PRESENT_SRC_KHR,
        _ => return None,
    })
}

/// The tiling of an image as Skia names it.
#[cfg(ferro_skia_vulkan)]
pub(crate) fn sk_image_tiling(tiling: u32) -> Option<vk::ImageTiling> {
    use ferroui_vulkan::ash::vk::ImageTiling;
    match ImageTiling::from_raw(tiling as i32) {
        ImageTiling::OPTIMAL => Some(vk::ImageTiling::OPTIMAL),
        ImageTiling::LINEAR => Some(vk::ImageTiling::LINEAR),
        _ => None,
    }
}

/// The image of a Vulkan context as Skia takes it.
///
/// # Panics
/// Panics when the format, the layout or the tiling of the image is one the
/// backend does not render to.
#[cfg(ferro_skia_vulkan)]
pub(crate) fn sk_image_info(info: &VulkanImageInfo, current_queue_family: u32) -> vk::ImageInfo {
    let format = sk_format(info.format).unwrap_or_else(|| panic!("Unsupported image format {}", info.format));
    let layout =
        sk_image_layout(info.layout).unwrap_or_else(|| panic!("Unsupported image layout {}", info.layout));
    let tiling =
        sk_image_tiling(info.tiling).unwrap_or_else(|| panic!("Unsupported image tiling {}", info.tiling));

    // SAFETY: the memory handle is that of the device the image belongs to;
    // the image and its memory outlive the Skia objects made from the value,
    // which the callers release before they end the session of the image or
    // dispose the imported image.
    let alloc = unsafe {
        vk::Alloc::from_device_memory(info.memory_handle as usize as _, 0, info.memory_size, vk::AllocFlag::empty())
    };
    // SAFETY: as above.
    let mut image_info = unsafe {
        vk::ImageInfo::new(
            info.handle as usize as _,
            alloc,
            tiling,
            layout,
            format,
            info.level_count,
            current_queue_family,
            None,
            if info.is_protected { Protected::Yes } else { Protected::No },
            None,
        )
    };
    image_info.image_usage_flags = info.usage_flags;
    image_info.sample_count = info.sample_count;
    image_info
}

#[cfg(all(test, ferro_skia_vulkan))]
mod tests;
