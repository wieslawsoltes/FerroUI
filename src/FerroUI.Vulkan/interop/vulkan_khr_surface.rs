use crate::{
    IVulkanKhrSurfacePlatformSurface, IVulkanPlatformGraphicsContext, VulkanError, VulkanResultExtensions,
};
use ash::vk;
use ferroui_base::PixelSize;
use std::cell::Cell;
use std::rc::Rc;

/// The `VkSurfaceKHR` of a platform surface.
pub(crate) struct VulkanKhrSurface {
    context: Rc<dyn IVulkanPlatformGraphicsContext>,
    surface_info: Rc<dyn IVulkanKhrSurfacePlatformSurface>,
    handle: Cell<vk::SurfaceKHR>,
}

impl VulkanKhrSurface {
    pub fn new(
        context: Rc<dyn IVulkanPlatformGraphicsContext>,
        surface_info: Rc<dyn IVulkanKhrSurfacePlatformSurface>,
    ) -> Self {
        let handle = vk::Handle::from_raw(surface_info.create_surface(&*context));
        Self { context, surface_info, handle: Cell::new(handle) }
    }

    pub fn handle(&self) -> vk::SurfaceKHR {
        self.handle.get()
    }

    pub fn can_surface_present(&self) -> Result<bool, VulkanError> {
        self.context
            .instance_api()
            .get_physical_device_surface_support_khr(
                self.context.physical_device_handle(),
                self.context.graphics_queue_family_index(),
                self.handle.get(),
            )
            .throw_on_error("vkGetPhysicalDeviceSurfaceSupportKHR")
    }

    pub fn get_surface_format(&self) -> Result<vk::SurfaceFormatKHR, VulkanError> {
        let surface_formats = self
            .context
            .instance_api()
            .get_physical_device_surface_formats_khr(self.context.physical_device_handle(), self.handle.get())
            .throw_on_error("vkGetPhysicalDeviceSurfaceFormatsKHR")?;

        select_surface_format(&surface_formats)
    }

    pub fn size(&self) -> PixelSize {
        self.surface_info.size()
    }

    pub fn dispose(&self) {
        if self.handle.get() != vk::SurfaceKHR::null() {
            self.context.instance_api().destroy_surface_khr(self.context.instance_handle(), self.handle.get());
        }
        self.handle.set(vk::SurfaceKHR::null());
    }
}

/// The format a swap chain is created with, among the formats a surface
/// supports: BGRA with 8 bits per channel in the sRGB colour space when the
/// surface has it or leaves the choice open, else its first format.
pub(crate) fn select_surface_format(
    surface_formats: &[vk::SurfaceFormatKHR],
) -> Result<vk::SurfaceFormatKHR, VulkanError> {
    if surface_formats.is_empty() {
        return Err(VulkanError::new("vkGetPhysicalDeviceSurfaceFormatsKHR returned 0 formats"));
    }

    if surface_formats.len() == 1 && surface_formats[0].format == vk::Format::UNDEFINED {
        return Ok(vk::SurfaceFormatKHR {
            format: vk::Format::B8G8R8A8_UNORM,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        });
    }
    for surface_format in surface_formats {
        if surface_format.format == vk::Format::B8G8R8A8_UNORM
            && surface_format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return Ok(*surface_format);
        }
    }

    Ok(surface_formats[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format(format: vk::Format, color_space: vk::ColorSpaceKHR) -> vk::SurfaceFormatKHR {
        vk::SurfaceFormatKHR { format, color_space }
    }

    #[test]
    fn no_format_is_a_failure() {
        let error = select_surface_format(&[]).unwrap_err();
        assert_eq!(error.message(), "vkGetPhysicalDeviceSurfaceFormatsKHR returned 0 formats");
    }

    #[test]
    fn an_open_choice_is_bgra_in_srgb() {
        let chosen =
            select_surface_format(&[format(vk::Format::UNDEFINED, vk::ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT)])
                .unwrap();
        assert_eq!(chosen, format(vk::Format::B8G8R8A8_UNORM, vk::ColorSpaceKHR::SRGB_NONLINEAR));
    }

    #[test]
    fn bgra_in_srgb_is_preferred_wherever_it_stands() {
        let formats = [
            format(vk::Format::R8G8B8A8_UNORM, vk::ColorSpaceKHR::SRGB_NONLINEAR),
            format(vk::Format::B8G8R8A8_SRGB, vk::ColorSpaceKHR::SRGB_NONLINEAR),
            format(vk::Format::B8G8R8A8_UNORM, vk::ColorSpaceKHR::SRGB_NONLINEAR),
        ];
        assert_eq!(select_surface_format(&formats).unwrap(), formats[2]);
    }

    #[test]
    fn otherwise_the_first_format_is_taken() {
        let formats = [
            format(vk::Format::R8G8B8A8_SRGB, vk::ColorSpaceKHR::SRGB_NONLINEAR),
            format(vk::Format::B8G8R8A8_UNORM, vk::ColorSpaceKHR::DISPLAY_P3_NONLINEAR_EXT),
        ];
        assert_eq!(select_surface_format(&formats).unwrap(), formats[0]);
        // Two formats, the first undefined: not the open choice.
        let formats = [
            format(vk::Format::UNDEFINED, vk::ColorSpaceKHR::SRGB_NONLINEAR),
            format(vk::Format::R8G8B8A8_UNORM, vk::ColorSpaceKHR::SRGB_NONLINEAR),
        ];
        assert_eq!(select_surface_format(&formats).unwrap(), formats[0]);
    }
}
