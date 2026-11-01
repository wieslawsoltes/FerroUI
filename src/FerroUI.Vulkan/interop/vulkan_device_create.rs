use super::VulkanDevice;
use crate::unmanaged_interop::vulkan_global_api::fixed_name;
use crate::unmanaged_interop::{Utf8BufferArray, VulkanInstanceApi};
use crate::{
    IVulkanDevice, IVulkanInstance, VulkanDeviceCreationOptions, VulkanError, VulkanExternalObjectsFeature,
    VulkanPlatformSpecificOptions, VulkanResultExtensions,
};
use ash::vk;
use std::sync::Arc;

const VK_KHR_SWAPCHAIN: &str = "VK_KHR_swapchain";

/// A physical device the framework can render with, and the queue family
/// it would use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DeviceInfo {
    pub physical_device: vk::PhysicalDevice,
    pub queue_family_index: u32,
    pub type_: vk::PhysicalDeviceType,
    pub extensions: Vec<String>,
    pub queue_count: u32,
}

impl VulkanDevice {
    /// Creates a device on the first suitable physical device of the
    /// instance (the first suitable discrete one when the options prefer
    /// it), with all queues of the first queue family that has the required
    /// capabilities and can present to the surface the platform probes
    /// with.
    pub fn create(
        instance: Arc<dyn IVulkanInstance>,
        options: &VulkanDeviceCreationOptions,
        platform_options: &VulkanPlatformSpecificOptions,
    ) -> Result<Arc<dyn IVulkanDevice>, VulkanError> {
        let api = VulkanInstanceApi::new(instance.clone());
        let vk_instance: vk::Instance = vk::Handle::from_raw(instance.handle() as u64);
        let devices = api.enumerate_physical_devices(vk_instance).throw_on_error("vkEnumeratePhysicalDevices")?;

        if devices.is_empty() {
            return Err(VulkanError::new("No devices found"));
        }

        let surface_for_probe_ptr =
            platform_options.device_check_surface_factory.as_ref().map(|factory| factory(&instance)).transpose()?;
        let surface_for_probe: Option<vk::SurfaceKHR> =
            surface_for_probe_ptr.filter(|surface| *surface != 0).map(vk::Handle::from_raw);

        let failure = std::cell::RefCell::new(None);
        let compatible_device = select_device(
            devices.iter().map(|device| match Self::check_device(&api, *device, options, surface_for_probe) {
                Ok(info) => info,
                Err(error) => {
                    failure.borrow_mut().get_or_insert(error);
                    None
                }
            }),
            options.prefer_discrete_gpu,
            || failure.borrow().is_some(),
        );
        if let Some(failure) = failure.into_inner() {
            return Err(failure);
        }

        let dev = compatible_device.ok_or_else(|| VulkanError::new("No compatible devices found"))?;

        let queue_priorities = vec![1f32; dev.queue_count as usize];

        let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(dev.queue_family_index)
            .queue_priorities(&queue_priorities)];

        let enabled_extensions = device_extensions(
            &options.device_extensions,
            VulkanExternalObjectsFeature::required_device_extensions(),
            &dev.extensions,
            options.require_swapchain_extension,
        );

        let p_enabled_extensions = Utf8BufferArray::new(&enabled_extensions);
        let create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(p_enabled_extensions.as_slice());

        let created_device = api.create_device(dev.physical_device, &create_info).throw_on_error("vkCreateDevice")?;

        let created_queue = api.get_device_queue(created_device, dev.queue_family_index, 0);

        Ok(VulkanDevice::new(
            api,
            created_device,
            dev.physical_device,
            created_queue,
            dev.queue_family_index,
            enabled_extensions,
        ))
    }

    fn get_device_extensions(instance: &VulkanInstanceApi, physical_device: vk::PhysicalDevice) -> Vec<String> {
        // A failure of the enumeration is a device without extensions, as in
        // the reference, which does not read the result.
        instance
            .enumerate_device_extension_properties(physical_device)
            .unwrap_or_default()
            .iter()
            .map(|properties| fixed_name(&properties.extension_name))
            .collect()
    }

    fn check_device(
        instance: &VulkanInstanceApi,
        physical_device: vk::PhysicalDevice,
        options: &VulkanDeviceCreationOptions,
        surface: Option<vk::SurfaceKHR>,
    ) -> Result<Option<DeviceInfo>, VulkanError> {
        let properties = instance.get_physical_device_properties(physical_device);

        let supported_extensions = Self::get_device_extensions(instance, physical_device);
        if options.require_swapchain_extension && !supported_extensions.iter().any(|name| name == VK_KHR_SWAPCHAIN) {
            return Ok(None);
        }

        let family_properties = instance.get_physical_device_queue_family_properties(physical_device);
        let family = select_queue_family(&family_properties, options.require_compute_bit, |c| match surface {
            Some(surface) => instance
                .get_physical_device_surface_support_khr(physical_device, c, surface)
                .throw_on_error("vkGetPhysicalDeviceSurfaceSupportKHR"),
            None => Ok(true),
        })?;

        Ok(family.map(|c| DeviceInfo {
            physical_device,
            extensions: supported_extensions,
            type_: properties.device_type,
            queue_family_index: c,
            queue_count: family_properties[c as usize].queue_count,
        }))
    }
}

/// The first queue family that has graphics (and compute, when required)
/// and that `supports_surface`.
pub(crate) fn select_queue_family(
    family_properties: &[vk::QueueFamilyProperties],
    require_compute_bit: bool,
    mut supports_surface: impl FnMut(u32) -> Result<bool, VulkanError>,
) -> Result<Option<u32>, VulkanError> {
    let mut required_flags = vk::QueueFlags::GRAPHICS;
    if require_compute_bit {
        required_flags |= vk::QueueFlags::COMPUTE;
    }

    for (c, family) in family_properties.iter().enumerate() {
        if (family.queue_flags & required_flags) != required_flags {
            continue;
        }
        if !supports_surface(c as u32)? {
            continue;
        }
        return Ok(Some(c as u32));
    }

    Ok(None)
}

/// The device to create among the checked physical devices, which are
/// checked in their order and only as far as needed: the first compatible
/// one, or with `prefer_discrete_gpu` the first compatible discrete one
/// when there is one. `stop` ends the walk early (a check failed).
pub(crate) fn select_device(
    devices: impl Iterator<Item = Option<DeviceInfo>>,
    prefer_discrete_gpu: bool,
    stop: impl Fn() -> bool,
) -> Option<DeviceInfo> {
    let mut compatible_device: Option<DeviceInfo> = None;
    let mut discrete_device: Option<DeviceInfo> = None;
    for info in devices {
        if stop() {
            break;
        }
        if let Some(info) = info {
            if info.type_ == vk::PhysicalDeviceType::DISCRETE_GPU && discrete_device.is_none() {
                discrete_device = Some(info.clone());
            }
            if compatible_device.is_none() {
                compatible_device = Some(info);
            }
        }

        if compatible_device.is_some() && (discrete_device.is_some() || !prefer_discrete_gpu) {
            break;
        }
    }

    if prefer_discrete_gpu && discrete_device.is_some() {
        compatible_device = discrete_device;
    }

    compatible_device
}

/// The device extensions to enable: those the options and the external
/// objects feature ask for that the device has, each once, and the swap
/// chain extension when it is required.
pub(crate) fn device_extensions(
    options_extensions: &[String],
    external_objects_extensions: &[&str],
    supported_extensions: &[String],
    require_swapchain_extension: bool,
) -> Vec<String> {
    let mut enabled_extensions: Vec<String> = Vec::new();
    let asked = options_extensions.iter().map(String::as_str).chain(external_objects_extensions.iter().copied());
    for name in asked {
        if supported_extensions.iter().any(|supported| supported == name)
            && !enabled_extensions.iter().any(|enabled| enabled == name)
        {
            enabled_extensions.push(name.to_string());
        }
    }
    if require_swapchain_extension && !enabled_extensions.iter().any(|enabled| enabled == VK_KHR_SWAPCHAIN) {
        enabled_extensions.push(VK_KHR_SWAPCHAIN.to_string());
    }
    enabled_extensions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn family(flags: vk::QueueFlags, count: u32) -> vk::QueueFamilyProperties {
        vk::QueueFamilyProperties { queue_flags: flags, queue_count: count, ..Default::default() }
    }

    fn info(id: u64, type_: vk::PhysicalDeviceType) -> Option<DeviceInfo> {
        Some(DeviceInfo {
            physical_device: vk::Handle::from_raw(id),
            queue_family_index: 0,
            type_,
            extensions: Vec::new(),
            queue_count: 1,
        })
    }

    fn id(info: Option<DeviceInfo>) -> Option<u64> {
        info.map(|info| vk::Handle::as_raw(info.physical_device))
    }

    #[test]
    fn the_first_family_with_graphics_is_taken() {
        let families = [
            family(vk::QueueFlags::TRANSFER, 1),
            family(vk::QueueFlags::GRAPHICS | vk::QueueFlags::TRANSFER, 4),
            family(vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE, 2),
        ];
        assert_eq!(select_queue_family(&families, false, |_| Ok(true)).unwrap(), Some(1));
        assert_eq!(select_queue_family(&families, true, |_| Ok(true)).unwrap(), Some(2));
        assert_eq!(select_queue_family(&families[..1], false, |_| Ok(true)).unwrap(), None);
    }

    #[test]
    fn a_family_that_cannot_present_is_passed_over() {
        let families = [family(vk::QueueFlags::GRAPHICS, 1), family(vk::QueueFlags::GRAPHICS, 1)];
        let mut asked = Vec::new();
        let chosen = select_queue_family(&families, false, |c| {
            asked.push(c);
            Ok(c == 1)
        })
        .unwrap();
        assert_eq!(chosen, Some(1));
        assert_eq!(asked, [0, 1]);
        assert_eq!(select_queue_family(&families, false, |_| Ok(false)).unwrap(), None);
    }

    #[test]
    fn a_failure_of_the_surface_query_is_the_failure_of_the_check() {
        let families = [family(vk::QueueFlags::GRAPHICS, 1)];
        let error =
            select_queue_family(&families, false, |_| Err(VulkanError::new("lost"))).unwrap_err();
        assert_eq!(error.message(), "lost");
    }

    #[test]
    fn the_first_compatible_device_is_taken_and_the_rest_is_not_checked() {
        let mut checked = 0;
        let devices = [None, info(2, vk::PhysicalDeviceType::CPU), info(3, vk::PhysicalDeviceType::DISCRETE_GPU)];
        let chosen = select_device(devices.into_iter().inspect(|_| checked += 1), false, || false);
        assert_eq!(id(chosen), Some(2));
        assert_eq!(checked, 2);
    }

    #[test]
    fn a_discrete_device_is_preferred_when_asked_for() {
        let devices = || {
            [
                info(1, vk::PhysicalDeviceType::INTEGRATED_GPU),
                info(2, vk::PhysicalDeviceType::DISCRETE_GPU),
                info(3, vk::PhysicalDeviceType::DISCRETE_GPU),
            ]
            .into_iter()
        };
        assert_eq!(id(select_device(devices(), true, || false)), Some(2));
        assert_eq!(id(select_device(devices(), false, || false)), Some(1));
        // Without a discrete one the first compatible one stays.
        let devices = [info(1, vk::PhysicalDeviceType::INTEGRATED_GPU), info(2, vk::PhysicalDeviceType::CPU)];
        assert_eq!(id(select_device(devices.into_iter(), true, || false)), Some(1));
    }

    #[test]
    fn no_compatible_device_is_none() {
        assert_eq!(select_device([None, None].into_iter(), true, || false), None);
        assert_eq!(select_device(std::iter::empty(), false, || false), None);
    }

    #[test]
    fn asked_extensions_are_enabled_when_the_device_has_them() {
        let supported: Vec<String> =
            ["VK_KHR_swapchain", "VK_KHR_external_memory", "VK_B", "VK_KHR_dedicated_allocation"]
                .iter()
                .map(|s| s.to_string())
                .collect();
        let options = vec!["VK_A".to_string(), "VK_B".to_string(), "VK_B".to_string()];
        let external = ["VK_KHR_external_memory", "VK_KHR_external_semaphore", "VK_KHR_dedicated_allocation"];
        assert_eq!(
            device_extensions(&options, &external, &supported, true),
            ["VK_B", "VK_KHR_external_memory", "VK_KHR_dedicated_allocation", "VK_KHR_swapchain"]
        );
        assert_eq!(
            device_extensions(&options, &external, &supported, false),
            ["VK_B", "VK_KHR_external_memory", "VK_KHR_dedicated_allocation"]
        );
    }

    #[test]
    fn the_swap_chain_extension_is_enabled_once() {
        let supported = vec!["VK_KHR_swapchain".to_string()];
        let options = vec!["VK_KHR_swapchain".to_string()];
        assert_eq!(device_extensions(&options, &[], &supported, true), ["VK_KHR_swapchain"]);
        // Required and not reported: asked of the device anyway, which then refuses.
        assert_eq!(device_extensions(&[], &[], &[], true), ["VK_KHR_swapchain"]);
    }
}
