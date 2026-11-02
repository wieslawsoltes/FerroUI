use super::VulkanDebugLogger;
use crate::unmanaged_interop::vulkan_global_api::{fixed_name, proc_loader};
use crate::unmanaged_interop::{Utf8BufferArray, VulkanGlobalApi, VulkanInstanceApi};
use crate::{
    IVulkanInstance, VkGetInstanceProcAddressDelegate, VulkanError, VulkanExternalObjectsFeature, VulkanHelpers,
    VulkanInstanceCreationOptions, VulkanPlatformSpecificOptions, VulkanResultExtensions,
};
use ash::vk;
use std::collections::HashSet;
use std::ffi::CString;
use std::sync::Arc;

/// The name the instance gives Vulkan for the application, unless the
/// options name it, and for the engine.
const DEFAULT_NAME: &str = "FerroUI";

/// An instance the platform graphics created.
pub(crate) struct VulkanInstance {
    handle: vk::Instance,
    get_proc_address: VkGetInstanceProcAddressDelegate,
    /// For `vkGetDeviceProcAddr`.
    fp: ash::InstanceFnV1_0,
    enabled_extensions: Vec<String>,
}

impl VulkanInstance {
    pub fn new(
        handle: vk::Instance,
        get_proc_address: VkGetInstanceProcAddressDelegate,
        enabled_extensions: Vec<String>,
    ) -> Self {
        let raw = vk::Handle::as_raw(handle) as isize;
        let fp = ash::InstanceFnV1_0::load(proc_loader(|name| get_proc_address(raw, name)));
        Self { handle, get_proc_address, fp, enabled_extensions }
    }

    /// Creates an instance with the extensions and layers the options and
    /// the platform ask for, and with the debug messenger when the options
    /// ask for it.
    pub fn create(
        options: &VulkanInstanceCreationOptions,
        platform_options: &VulkanPlatformSpecificOptions,
    ) -> Result<Arc<dyn IVulkanInstance>, VulkanError> {
        let get_proc_address = options
            .custom_get_proc_address_delegate
            .clone()
            .or_else(|| platform_options.get_proc_address_delegate.clone())
            .ok_or_else(|| VulkanError::new("No VkGetInstanceProcAddr provided"))?;

        let name = c_string(options.application_name.as_deref().unwrap_or(DEFAULT_NAME));
        let engine_name = c_string(DEFAULT_NAME);

        let gapi = VulkanGlobalApi::try_new(get_proc_address.clone())?;

        let supported_extensions = Self::get_supported_extensions(&gapi)?;
        let app_info = vk::ApplicationInfo::default()
            .api_version(VulkanHelpers::make_version_of(options.vulkan_version))
            .application_version(VulkanHelpers::make_version(1, 0, 0))
            .engine_version(VulkanHelpers::make_version(1, 0, 0))
            .application_name(&name)
            .engine_name(&engine_name);

        let enabled_extensions = instance_extensions(options, platform_options, &supported_extensions);

        let mut enabled_layers = options.enabled_layers.clone();
        if options.use_debug && Self::is_layer_available(&gapi, "VK_LAYER_KHRONOS_validation")? {
            enabled_layers.push("VK_LAYER_KHRONOS_validation".to_string());
        }

        let enabled_extension_buffers = Utf8BufferArray::new(distinct_names(&enabled_extensions));
        let enabled_layers_buffers = Utf8BufferArray::new(distinct_names(&enabled_layers));

        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_layer_names(enabled_layers_buffers.as_slice())
            .enabled_extension_names(enabled_extension_buffers.as_slice());

        let p_instance = gapi.vk_create_instance(&create_info).throw_on_error("vkCreateInstance")?;

        let instance: Arc<dyn IVulkanInstance> =
            Arc::new(VulkanInstance::new(p_instance, get_proc_address, enabled_extensions));
        let instance_api = VulkanInstanceApi::new(instance.clone());

        if options.use_debug {
            let debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                        | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                        | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(VulkanDebugLogger::callback_ptr());

            // The messenger lives as long as the instance, which is never
            // destroyed. Without the debug extension the entry point is
            // missing, and nothing is created.
            if instance.enabled_extensions().iter().any(|name| name == "VK_EXT_debug_utils") {
                let _ = instance_api.create_debug_utils_messenger_ext(p_instance, &debug_create_info);
            }
        }

        Ok(instance)
    }

    fn is_layer_available(api: &VulkanGlobalApi, layer_name: &str) -> Result<bool, VulkanError> {
        let layer_properties =
            api.enumerate_instance_layer_properties().throw_on_error("vkEnumerateInstanceLayerProperties")?;

        Ok(layer_properties.iter().any(|properties| fixed_name(&properties.layer_name) == layer_name))
    }

    fn get_supported_extensions(api: &VulkanGlobalApi) -> Result<HashSet<String>, VulkanError> {
        let properties = api
            .vk_enumerate_instance_extension_properties()
            .throw_on_error("vkEnumerateInstanceExtensionProperties")?;
        Ok(properties.iter().map(|properties| fixed_name(&properties.extension_name)).collect())
    }
}

impl IVulkanInstance for VulkanInstance {
    fn handle(&self) -> isize {
        vk::Handle::as_raw(self.handle) as isize
    }

    fn get_instance_proc_address(&self, instance: isize, name: &str) -> isize {
        (self.get_proc_address)(instance, name)
    }

    fn get_device_proc_address(&self, device: isize, name: &str) -> isize {
        let buf = c_string(name);
        // SAFETY: the name is a zero-terminated string; the device is one of
        // this instance, as the contract of the member says.
        let address =
            unsafe { (self.fp.get_device_proc_addr)(vk::Handle::from_raw(device as u64), buf.as_ptr()) };
        address.map_or(0, |address| address as usize as isize)
    }

    fn enabled_extensions(&self) -> Vec<String> {
        self.enabled_extensions.clone()
    }
}

/// The zero-terminated form of a string, cut at its first zero character.
fn c_string(s: &str) -> CString {
    let end = s.find('\0').unwrap_or(s.len());
    CString::new(&s[..end]).expect("the string was cut at its first zero character")
}

/// The names of a list that are not blank, each once, in their order.
fn distinct_names(names: &[String]) -> Vec<&str> {
    let mut seen = HashSet::new();
    names.iter().map(String::as_str).filter(|x| !x.trim().is_empty() && seen.insert(*x)).collect()
}

/// The instance extensions to enable, each once, in the order in which they
/// are asked for: those of the options, those of the platform,
/// `VK_KHR_surface` unless the options do without, the debug extension when
/// the options ask for it and the system has it, and the extensions of the
/// external objects feature when the system has all of them.
pub(crate) fn instance_extensions(
    options: &VulkanInstanceCreationOptions,
    platform_options: &VulkanPlatformSpecificOptions,
    supported_extensions: &HashSet<String>,
) -> Vec<String> {
    let mut enabled_extensions: Vec<String> = Vec::new();
    let add = |enabled_extensions: &mut Vec<String>, name: &str| {
        if !enabled_extensions.iter().any(|x| x == name) {
            enabled_extensions.push(name.to_string());
        }
    };
    for name in options.instance_extensions.iter().chain(platform_options.required_instance_extensions.iter()) {
        add(&mut enabled_extensions, name);
    }

    if options.require_surface_extension {
        add(&mut enabled_extensions, "VK_KHR_surface");
    }

    let add_extensions_if_supported = |enabled_extensions: &mut Vec<String>, names: &[&str]| {
        if names.iter().all(|n| supported_extensions.contains(*n)) {
            for n in names {
                add(enabled_extensions, n);
            }
        }
    };

    if options.use_debug {
        add_extensions_if_supported(&mut enabled_extensions, &["VK_EXT_debug_utils"]);
    }

    add_extensions_if_supported(&mut enabled_extensions, VulkanExternalObjectsFeature::REQUIRED_INSTANCE_EXTENSIONS);

    enabled_extensions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supported(names: &[&str]) -> HashSet<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn the_surface_extension_is_required_by_default() {
        let extensions = instance_extensions(&Default::default(), &Default::default(), &supported(&[]));
        assert_eq!(extensions, ["VK_KHR_surface"]);

        let options = VulkanInstanceCreationOptions { require_surface_extension: false, ..Default::default() };
        assert!(instance_extensions(&options, &Default::default(), &supported(&[])).is_empty());
    }

    #[test]
    fn the_extensions_of_the_options_and_of_the_platform_come_first_and_once() {
        let options = VulkanInstanceCreationOptions {
            instance_extensions: vec!["VK_A".into(), "VK_KHR_xlib_surface".into()],
            ..Default::default()
        };
        let platform = VulkanPlatformSpecificOptions {
            required_instance_extensions: vec!["VK_KHR_xlib_surface".into(), "VK_KHR_surface".into()],
            ..Default::default()
        };
        // Whether the system has them is not asked: creating the instance fails without them.
        let extensions = instance_extensions(&options, &platform, &supported(&[]));
        assert_eq!(extensions, ["VK_A", "VK_KHR_xlib_surface", "VK_KHR_surface"]);
    }

    #[test]
    fn the_debug_extension_is_added_when_asked_for_and_supported() {
        let debug = VulkanInstanceCreationOptions { use_debug: true, ..Default::default() };
        let with = supported(&["VK_EXT_debug_utils"]);
        assert_eq!(instance_extensions(&debug, &Default::default(), &with), ["VK_KHR_surface", "VK_EXT_debug_utils"]);
        assert_eq!(instance_extensions(&debug, &Default::default(), &supported(&[])), ["VK_KHR_surface"]);
        assert_eq!(instance_extensions(&Default::default(), &Default::default(), &with), ["VK_KHR_surface"]);
    }

    #[test]
    fn the_external_objects_extensions_are_added_only_together() {
        let all = supported(&[
            "VK_KHR_get_physical_device_properties2",
            "VK_KHR_external_memory_capabilities",
            "VK_KHR_external_semaphore_capabilities",
        ]);
        assert_eq!(
            instance_extensions(&Default::default(), &Default::default(), &all),
            [
                "VK_KHR_surface",
                "VK_KHR_get_physical_device_properties2",
                "VK_KHR_external_memory_capabilities",
                "VK_KHR_external_semaphore_capabilities"
            ]
        );
        let two = supported(&["VK_KHR_get_physical_device_properties2", "VK_KHR_external_memory_capabilities"]);
        assert_eq!(instance_extensions(&Default::default(), &Default::default(), &two), ["VK_KHR_surface"]);
    }

    #[test]
    fn blank_and_repeated_names_are_not_given_to_vulkan() {
        let names = vec!["VK_A".to_string(), " ".to_string(), String::new(), "VK_B".to_string(), "VK_A".to_string()];
        assert_eq!(distinct_names(&names), ["VK_A", "VK_B"]);
    }

    #[test]
    fn without_a_way_to_the_entry_points_no_instance_is_created() {
        let error = VulkanInstance::create(&Default::default(), &Default::default()).err().unwrap();
        assert_eq!(error.message(), "No VkGetInstanceProcAddr provided");
    }

    #[test]
    fn the_names_given_to_vulkan_are_those_of_the_port() {
        assert_eq!(DEFAULT_NAME, "FerroUI");
    }
}
