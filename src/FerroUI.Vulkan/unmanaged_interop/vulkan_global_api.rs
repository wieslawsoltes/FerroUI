use crate::VkGetInstanceProcAddressDelegate;
use ash::vk;
use std::ffi::{c_void, CStr};
use std::ptr;

/// The entry points of Vulkan that need no instance, found through the
/// delegate of a platform.
pub struct VulkanGlobalApi {
    vk_get_proc_address: VkGetInstanceProcAddressDelegate,
    fp: ash::EntryFnV1_0,
}

impl VulkanGlobalApi {
    /// Resolves the entry points. One the delegate does not know is replaced
    /// by a function that stops the program when it is called.
    pub fn new(vk_get_proc_address: VkGetInstanceProcAddressDelegate) -> Self {
        let fp = ash::EntryFnV1_0::load(proc_loader(|name| vk_get_proc_address(0, name)));
        Self { vk_get_proc_address, fp }
    }

    /// Resolves the entry points, or names the first one the lookup does not
    /// know: a lookup that is not that of a Vulkan loader (or a loader that
    /// could not be opened) fails here, where the reference fails to fill
    /// its table.
    pub fn try_new(vk_get_proc_address: VkGetInstanceProcAddressDelegate) -> Result<Self, crate::VulkanError> {
        for name in ["vkEnumerateInstanceLayerProperties", "vkCreateInstance", "vkEnumerateInstanceExtensionProperties"] {
            if vk_get_proc_address(0, name) == 0 {
                return Err(crate::VulkanError::new(format!("Unable to find the entry point {name}")));
            }
        }
        Ok(Self::new(vk_get_proc_address))
    }

    /// The address of an entry point of `instance`.
    pub fn get_proc_address(&self, instance: vk::Instance, name: &str) -> isize {
        (self.vk_get_proc_address)(vk::Handle::as_raw(instance) as isize, name)
    }

    /// `vkEnumerateInstanceLayerProperties`: the layers of the system.
    pub fn enumerate_instance_layer_properties(&self) -> Result<Vec<vk::LayerProperties>, vk::Result> {
        // SAFETY: the function writes at most `count` elements into the
        // buffer `read_into_vec` gives it.
        read_into_vec(|count, data| unsafe { (self.fp.enumerate_instance_layer_properties)(count, data) })
    }

    /// `vkCreateInstance`.
    pub fn vk_create_instance(&self, create_info: &vk::InstanceCreateInfo<'_>) -> Result<vk::Instance, vk::Result> {
        let mut instance = vk::Instance::null();
        // SAFETY: the lifetime of the structure covers what it points to,
        // and the result is written to a local.
        let result = unsafe { (self.fp.create_instance)(create_info, ptr::null(), &mut instance) };
        match result {
            vk::Result::SUCCESS => Ok(instance),
            error => Err(error),
        }
    }

    /// `vkEnumerateInstanceExtensionProperties` without a layer: the
    /// instance extensions of the system.
    pub fn vk_enumerate_instance_extension_properties(&self) -> Result<Vec<vk::ExtensionProperties>, vk::Result> {
        // SAFETY: as for the layers.
        read_into_vec(|count, data| unsafe {
            (self.fp.enumerate_instance_extension_properties)(ptr::null(), count, data)
        })
    }
}

/// The form in which the function tables ask for an entry point, over a
/// lookup by name.
pub(crate) fn proc_loader<'a>(lookup: impl Fn(&str) -> isize + 'a) -> impl FnMut(&CStr) -> *const c_void + 'a {
    move |name| name.to_str().map_or(ptr::null(), |name| lookup(name) as *const c_void)
}

/// The two calls of a Vulkan enumeration: the count, then the elements.
///
/// The reference makes the two calls once; between them the count may grow,
/// which shows as `VK_INCOMPLETE` and is asked again here.
pub(crate) fn read_into_vec<T: Default + Clone>(
    mut call: impl FnMut(*mut u32, *mut T) -> vk::Result,
) -> Result<Vec<T>, vk::Result> {
    loop {
        let mut count = 0u32;
        match call(&mut count, ptr::null_mut()) {
            vk::Result::SUCCESS => {}
            error => return Err(error),
        }
        let mut data = vec![T::default(); count as usize];
        match call(&mut count, data.as_mut_ptr()) {
            vk::Result::SUCCESS => {
                data.truncate(count as usize);
                return Ok(data);
            }
            vk::Result::INCOMPLETE => continue,
            error => return Err(error),
        }
    }
}

/// The name in a fixed array of characters of a Vulkan structure.
pub(crate) fn fixed_name(name: &[std::ffi::c_char]) -> String {
    let bytes: Vec<u8> = name.iter().take_while(|c| **c != 0).map(|c| *c as u8).collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_enumeration_asks_for_the_count_and_then_for_the_elements() {
        let mut calls = Vec::new();
        let result: Vec<u32> = read_into_vec(|count, data: *mut u32| {
            calls.push(data.is_null());
            // SAFETY: the pointers are those of the function under test.
            unsafe {
                if data.is_null() {
                    *count = 3;
                } else {
                    assert_eq!(*count, 3);
                    for i in 0..3 {
                        *data.add(i) = 10 + i as u32;
                    }
                }
            }
            vk::Result::SUCCESS
        })
        .unwrap();
        assert_eq!(result, [10, 11, 12]);
        assert_eq!(calls, [true, false]);
    }

    #[test]
    fn an_enumeration_that_shrinks_is_cut_and_one_that_grows_is_asked_again() {
        let mut round = 0;
        let result: Vec<u32> = read_into_vec(|count, data: *mut u32| {
            // SAFETY: the pointers are those of the function under test.
            unsafe {
                if data.is_null() {
                    round += 1;
                    *count = if round == 1 { 1 } else { 4 };
                    return vk::Result::SUCCESS;
                }
                if round == 1 {
                    return vk::Result::INCOMPLETE;
                }
                *count = 2;
                *data = 7;
                *data.add(1) = 8;
            }
            vk::Result::SUCCESS
        })
        .unwrap();
        assert_eq!(result, [7, 8]);
    }

    #[test]
    fn a_failure_of_either_call_is_the_failure_of_the_enumeration() {
        let result: Result<Vec<u32>, _> = read_into_vec(|_, _| vk::Result::ERROR_OUT_OF_HOST_MEMORY);
        assert_eq!(result, Err(vk::Result::ERROR_OUT_OF_HOST_MEMORY));
    }

    #[test]
    fn a_fixed_name_ends_at_its_terminator() {
        let mut properties = vk::ExtensionProperties::default();
        for (i, b) in b"VK_KHR_surface".iter().enumerate() {
            properties.extension_name[i] = *b as std::ffi::c_char;
        }
        assert_eq!(fixed_name(&properties.extension_name), "VK_KHR_surface");
    }

    #[test]
    fn a_lookup_that_knows_no_entry_point_gives_no_table() {
        let error = VulkanGlobalApi::try_new(std::sync::Arc::new(|_, _| 0)).err().unwrap();
        assert_eq!(error.message(), "Unable to find the entry point vkEnumerateInstanceLayerProperties");
        let error = VulkanGlobalApi::try_new(std::sync::Arc::new(|_, name| (name != "vkCreateInstance") as isize))
            .err()
            .unwrap();
        assert_eq!(error.message(), "Unable to find the entry point vkCreateInstance");
    }

    #[test]
    fn the_global_entry_points_are_asked_without_an_instance() {
        use std::sync::{Arc, Mutex};
        let asked: Arc<Mutex<Vec<(isize, String)>>> = Arc::default();
        let log = asked.clone();
        let api = VulkanGlobalApi::new(Arc::new(move |instance, name| {
            log.lock().unwrap().push((instance, name.to_string()));
            0
        }));
        {
            let asked = asked.lock().unwrap();
            assert!(asked.iter().all(|(instance, _)| *instance == 0));
            let names: Vec<&str> = asked.iter().map(|(_, name)| name.as_str()).collect();
            assert_eq!(
                names,
                [
                    "vkCreateInstance",
                    "vkEnumerateInstanceExtensionProperties",
                    "vkEnumerateInstanceLayerProperties"
                ]
            );
        }
        assert_eq!(api.get_proc_address(vk::Handle::from_raw(42), "vkDestroyInstance"), 0);
        assert_eq!(asked.lock().unwrap().last().unwrap(), &(42, "vkDestroyInstance".to_string()));
    }
}
