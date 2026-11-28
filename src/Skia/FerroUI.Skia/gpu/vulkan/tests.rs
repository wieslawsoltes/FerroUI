//! The value tables of the module.

use super::{sk_format, sk_image_layout, sk_image_tiling, WrappedSurfaceKey};
use ferroui_base::PixelSize;
use ferroui_vulkan::ash::vk;
use ferroui_vulkan::VulkanImageInfo;
use skia_safe::{PixelGeometry, SurfaceProps, SurfacePropsFlags};

fn image(handle: u64, layout: vk::ImageLayout) -> VulkanImageInfo {
    VulkanImageInfo {
        handle,
        pixel_size: PixelSize::new(200, 120),
        format: vk::Format::B8G8R8A8_UNORM.as_raw() as u32,
        layout: layout.as_raw() as u32,
        level_count: 1,
        sample_count: 1,
        memory_handle: 7,
        memory_size: 200 * 120 * 4,
        view_handle: 9,
        ..Default::default()
    }
}

#[test]
fn a_wrapped_surface_is_kept_while_the_image_is_the_same_in_any_layout() {
    let props = SurfaceProps::default();
    let key = |info| WrappedSurfaceKey::new(info, true, false, props);
    let color = key(image(1, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL));
    assert_eq!(color, key(image(1, vk::ImageLayout::TRANSFER_SRC_OPTIMAL)));
    assert_eq!(color, key(image(1, vk::ImageLayout::UNDEFINED)));
}

#[test]
fn a_new_image_or_other_properties_wrap_a_new_surface() {
    let props = SurfaceProps::default();
    let info = image(1, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
    let key = WrappedSurfaceKey::new(info, true, false, props);
    // A new image: another handle, another memory, another size or format.
    assert_ne!(key, WrappedSurfaceKey::new(image(2, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL), true, false, props));
    let other_memory = VulkanImageInfo { memory_handle: 8, ..info };
    assert_ne!(key, WrappedSurfaceKey::new(other_memory, true, false, props));
    let other_size = VulkanImageInfo { pixel_size: PixelSize::new(201, 120), ..info };
    assert_ne!(key, WrappedSurfaceKey::new(other_size, true, false, props));
    let other_format = VulkanImageInfo { format: vk::Format::R8G8B8A8_UNORM.as_raw() as u32, ..info };
    assert_ne!(key, WrappedSurfaceKey::new(other_format, true, true, props));
    // The same image drawn another way.
    assert_ne!(key, WrappedSurfaceKey::new(info, false, false, props));
    let lcd = SurfaceProps::new(SurfacePropsFlags::default(), PixelGeometry::RGBH);
    assert_ne!(key, WrappedSurfaceKey::new(info, true, false, lcd));
}

#[test]
fn the_values_of_vulkan_map_to_the_names_of_skia() {
    use skia_safe::gpu::vk as sk;
    assert!(matches!(sk_format(vk::Format::R8G8B8A8_UNORM.as_raw() as u32), Some(sk::Format::R8G8B8A8_UNORM)));
    assert!(matches!(sk_format(vk::Format::B8G8R8A8_UNORM.as_raw() as u32), Some(sk::Format::B8G8R8A8_UNORM)));
    assert!(sk_format(vk::Format::R8G8B8A8_SRGB.as_raw() as u32).is_none());
    assert!(matches!(
        sk_image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL.as_raw() as u32),
        Some(sk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
    ));
    assert!(matches!(
        sk_image_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL.as_raw() as u32),
        Some(sk::ImageLayout::TRANSFER_SRC_OPTIMAL)
    ));
    assert!(matches!(sk_image_layout(0), Some(sk::ImageLayout::UNDEFINED)));
    assert!(matches!(
        sk_image_layout(vk::ImageLayout::PRESENT_SRC_KHR.as_raw() as u32),
        Some(sk::ImageLayout::PRESENT_SRC_KHR)
    ));
    assert!(sk_image_layout(123456).is_none());
    assert!(matches!(sk_image_tiling(0), Some(sk::ImageTiling::OPTIMAL)));
    assert!(matches!(sk_image_tiling(1), Some(sk::ImageTiling::LINEAR)));
    assert!(sk_image_tiling(77).is_none());
}
