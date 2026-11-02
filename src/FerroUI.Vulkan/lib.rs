//! The Vulkan platform contracts and the platform graphics over them.
//!
//! What a platform exposes so that a render backend can draw with Vulkan:
//! the instance and the device ([`IVulkanInstance`], [`IVulkanDevice`]), the
//! graphics context of a device ([`IVulkanPlatformGraphicsContext`]), the
//! surfaces a context presents to ([`IVulkanKhrSurfacePlatformSurface`],
//! [`IVulkanKhrSurfacePlatformSurfaceFactory`],
//! [`IVulkanRenderTargetPlatformSurface`]), the render target of a surface
//! and the session of a frame ([`IVulkanRenderTarget`],
//! [`IVulkanRenderSession`]), and the import of images and semaphores that
//! were created outside of a device
//! ([`IVulkanContextExternalObjectsFeature`]).
//!
//! And the implementation every windowing platform shares
//! ([`VulkanPlatformGraphics`]): it creates the instance (with the debug
//! messenger when asked), selects a physical device, creates the device with
//! its queue, and gives each thread that renders a context whose render
//! targets present through a swap chain. A platform adds three things
//! ([`VulkanPlatformSpecificOptions`]): the way to the entry points of the
//! Vulkan loader, the instance extensions of its window system, and the
//! factory that turns its windows into surfaces.
//!
//! These types belong to no render backend. The types, constants and
//! function tables of Vulkan are those of the [`ash`] crate, which is
//! exported again for the crates that implement or read the contracts; the
//! loader of the system is opened at run time ([`ash::Entry::load`]), and
//! nothing is linked.

#![deny(unsafe_op_in_unsafe_fn)]

mod i_vulkan_context_external_objects_feature;
mod i_vulkan_device;
mod i_vulkan_platform_surface;
mod i_vulkan_render_target;
mod interop;
pub mod unmanaged_interop;
mod vulkan_bindings;
mod vulkan_context;
mod vulkan_exception;
mod vulkan_external_objects_feature;
mod vulkan_image_info;
mod vulkan_khr_surface_render_target;
mod vulkan_options;
mod vulkan_platform_graphics;

#[cfg(test)]
mod live_tests;

pub use ash;

pub use i_vulkan_context_external_objects_feature::{
    IVulkanContextExternalObjectsFeature, IVulkanExternalImage, IVulkanExternalSemaphore,
};
pub use i_vulkan_device::{IVulkanDevice, IVulkanInstance, IVulkanPlatformGraphicsContext};
pub use i_vulkan_platform_surface::{
    try_get_vulkan_khr_surface, try_get_vulkan_render_target_surface, IVulkanKhrSurfacePlatformSurface,
    IVulkanKhrSurfacePlatformSurfaceFactory, IVulkanRenderTargetPlatformSurface,
};
pub use i_vulkan_render_target::{IVulkanRenderSession, IVulkanRenderTarget};
pub use vulkan_bindings::VulkanHelpers;
pub use vulkan_exception::{VulkanError, VulkanExceptionExtensions, VulkanResultExtensions};
#[allow(deprecated)]
pub use vulkan_exception::VulkanException;
pub use vulkan_image_info::VulkanImageInfo;
pub use vulkan_options::{
    Version, VkGetInstanceProcAddressDelegate, VulkanDeviceCheckSurfaceFactory, VulkanDeviceCreationOptions,
    VulkanInstanceCreationOptions, VulkanOptions, VulkanPlatformSpecificOptions,
};
pub use vulkan_platform_graphics::{IVulkanDeviceFactory, VulkanPlatformGraphics};

pub(crate) use vulkan_external_objects_feature::VulkanExternalObjectsFeature;
pub(crate) use vulkan_khr_surface_render_target::VulkanKhrRenderTarget;
