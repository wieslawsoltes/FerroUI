use crate::interop::{VulkanCommandBufferPool, VulkanImageBase, VulkanMemoryHelper, VulkanSemaphore};
use crate::{
    IVulkanContextExternalObjectsFeature, IVulkanExternalImage, IVulkanExternalSemaphore,
    IVulkanPlatformGraphicsContext, VulkanError, VulkanExceptionExtensions, VulkanImageInfo,
    VulkanResultExtensions,
};
use ash::vk;
use ferroui_base::platform::{
    IPlatformHandle, KnownPlatformGraphicsExternalImageHandleTypes, KnownPlatformGraphicsExternalSemaphoreHandleTypes,
    PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties,
};
use ferroui_base::rendering::composition::CompositionGpuImportedImageSynchronizationCapabilities;
use ferroui_base::PixelSize;
use std::cell::RefCell;
use std::rc::Rc;

/// Imports images and semaphores into the device of a context, through the
/// external memory and external semaphore extensions of Vulkan.
pub(crate) struct VulkanExternalObjectsFeature {
    context: Rc<dyn IVulkanPlatformGraphicsContext>,
    pool: Rc<VulkanCommandBufferPool>,
    supported_image_handle_types: Vec<String>,
    supported_semaphore_types: Vec<String>,
    device_uuid: Option<Vec<u8>>,
    device_luid: Option<Vec<u8>>,
}

const REQUIRED_COMMON_DEVICE_EXTENSIONS: [&str; 3] =
    ["VK_KHR_external_memory", "VK_KHR_external_semaphore", "VK_KHR_dedicated_allocation"];

const REQUIRED_LINUX_DEVICE_EXTENSIONS: [&str; 5] = [
    REQUIRED_COMMON_DEVICE_EXTENSIONS[0],
    REQUIRED_COMMON_DEVICE_EXTENSIONS[1],
    REQUIRED_COMMON_DEVICE_EXTENSIONS[2],
    "VK_KHR_external_semaphore_fd",
    "VK_KHR_external_memory_fd",
];

const REQUIRED_WIN32_DEVICE_EXTENSIONS: [&str; 5] = [
    REQUIRED_COMMON_DEVICE_EXTENSIONS[0],
    REQUIRED_COMMON_DEVICE_EXTENSIONS[1],
    REQUIRED_COMMON_DEVICE_EXTENSIONS[2],
    "VK_KHR_external_semaphore_win32",
    "VK_KHR_external_memory_win32",
];

impl VulkanExternalObjectsFeature {
    /// The instance extensions the feature needs.
    pub const REQUIRED_INSTANCE_EXTENSIONS: &'static [&'static str] = &[
        "VK_KHR_get_physical_device_properties2",
        "VK_KHR_external_memory_capabilities",
        "VK_KHR_external_semaphore_capabilities",
    ];

    /// The device extensions the feature needs on this system: handles are
    /// file descriptors, or Windows handles on Windows.
    pub fn required_device_extensions() -> &'static [&'static str] {
        Self::required_device_extensions_for(cfg!(windows))
    }

    pub(crate) fn required_device_extensions_for(windows: bool) -> &'static [&'static str] {
        if windows {
            &REQUIRED_WIN32_DEVICE_EXTENSIONS
        } else {
            &REQUIRED_LINUX_DEVICE_EXTENSIONS
        }
    }

    /// The handle types of images and of semaphores the feature imports on
    /// a system.
    pub(crate) fn supported_types_for(windows: bool) -> (Vec<String>, Vec<String>) {
        if windows {
            //TODO: keyed muted
            (
                vec![
                    KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_NT_HANDLE.to_string(),
                    KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_KMT_HANDLE.to_string(),
                ],
                vec![
                    KnownPlatformGraphicsExternalSemaphoreHandleTypes::VULKAN_OPAQUE_NT_HANDLE.to_string(),
                    KnownPlatformGraphicsExternalSemaphoreHandleTypes::VULKAN_OPAQUE_KMT_HANDLE.to_string(),
                ],
            )
        } else {
            (
                vec![KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR.to_string()],
                vec![
                    KnownPlatformGraphicsExternalSemaphoreHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR.to_string(),
                ],
            )
        }
    }

    /// Creates the feature of `context`, whose instance and device have the
    /// required extensions.
    pub fn new(context: Rc<dyn IVulkanPlatformGraphicsContext>) -> Result<Self, VulkanError> {
        let (supported_image_handle_types, supported_semaphore_types) = Self::supported_types_for(cfg!(windows));

        let mut physical_device_id_properties = vk::PhysicalDeviceIDProperties::default();
        {
            let mut physical_device_properties2 =
                vk::PhysicalDeviceProperties2::default().push_next(&mut physical_device_id_properties);
            context
                .instance_api()
                .get_physical_device_properties2(context.physical_device_handle(), &mut physical_device_properties2);
        }

        let device_luid = non_zero(&physical_device_id_properties.device_luid);
        let device_uuid = non_zero(&physical_device_id_properties.device_uuid);

        let pool = VulkanCommandBufferPool::new(context.clone(), true)?;
        Ok(Self { context, pool, supported_image_handle_types, supported_semaphore_types, device_uuid, device_luid })
    }

    /// Frees what the feature created on the device. The reference leaves
    /// that to the end of the process.
    pub fn dispose(&self) {
        self.pool.dispose();
    }
}

/// The bytes of an identifier, unless all of them are zero.
fn non_zero(bytes: &[u8]) -> Option<Vec<u8>> {
    bytes.iter().any(|b| *b != 0).then(|| bytes.to_vec())
}

/// The Vulkan handle type of a semaphore handle of a kind the framework
/// names.
pub(crate) fn semaphore_handle_type(handle_descriptor: Option<&str>) -> Option<vk::ExternalSemaphoreHandleTypeFlags> {
    match handle_descriptor? {
        KnownPlatformGraphicsExternalSemaphoreHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR => {
            Some(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD)
        }
        KnownPlatformGraphicsExternalSemaphoreHandleTypes::VULKAN_OPAQUE_KMT_HANDLE => {
            Some(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32_KMT)
        }
        KnownPlatformGraphicsExternalSemaphoreHandleTypes::VULKAN_OPAQUE_NT_HANDLE => {
            Some(vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32)
        }
        _ => None,
    }
}

/// The Vulkan handle type of an image handle of a kind the framework
/// names.
pub(crate) fn memory_handle_type(handle_descriptor: Option<&str>) -> Option<vk::ExternalMemoryHandleTypeFlags> {
    match handle_descriptor? {
        KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR => {
            Some(vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD)
        }
        KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_KMT_HANDLE => {
            Some(vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32_KMT)
        }
        KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_NT_HANDLE => {
            Some(vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32)
        }
        _ => None,
    }
}

/// The Vulkan format of an image format the framework names.
pub(crate) fn image_format(format: PlatformGraphicsExternalImageFormat) -> vk::Format {
    match format {
        PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm => vk::Format::B8G8R8A8_UNORM,
        PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm => vk::Format::R8G8B8A8_UNORM,
    }
}

fn supports(types: &[String], handle_descriptor: Option<&str>) -> bool {
    handle_descriptor.is_some_and(|descriptor| types.iter().any(|x| x == descriptor))
}

impl IVulkanContextExternalObjectsFeature for VulkanExternalObjectsFeature {
    fn supported_image_handle_types(&self) -> Vec<String> {
        self.supported_image_handle_types.clone()
    }

    fn supported_semaphore_types(&self) -> Vec<String> {
        self.supported_semaphore_types.clone()
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        self.device_uuid.clone()
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        self.device_luid.clone()
    }

    fn get_synchronization_capabilities(
        &self,
        image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        if !supports(&self.supported_image_handle_types, Some(image_handle_type)) {
            panic!("Value does not fall within the expected range.");
        }
        //TODO: keyed muted
        CompositionGpuImportedImageSynchronizationCapabilities::SEMAPHORES
    }

    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IVulkanExternalImage> {
        if !supports(&self.supported_image_handle_types, handle.handle_descriptor()) {
            panic!("Specified method is not supported.");
        }

        match ImportedImage::new(self.context.clone(), self.pool.clone(), handle, properties) {
            Ok(image) => Rc::new(image),
            Err(error) => panic!("{error}"),
        }
    }

    fn import_semaphore(&self, handle: Rc<dyn IPlatformHandle>) -> Rc<dyn IVulkanExternalSemaphore> {
        if !supports(&self.supported_semaphore_types, handle.handle_descriptor()) {
            panic!("Specified method is not supported.");
        }

        let type_bit = semaphore_handle_type(handle.handle_descriptor())
            .unwrap_or_else(|| panic!("Specified method is not supported."));

        let semaphore = VulkanSemaphore::new(self.context.clone()).unwrap_or_else(|error| panic!("{error}"));
        let api = self.context.device_api();
        let device = self.context.device_handle();
        let imported = if handle.handle_descriptor()
            == Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR)
        {
            let info = vk::ImportSemaphoreFdInfoKHR::default()
                .fd(handle.handle() as i32)
                .handle_type(type_bit)
                .semaphore(semaphore.handle());
            api.import_semaphore_fd_khr(device, &info).throw_on_error("vkImportSemaphoreFdKHR")
        } else {
            let info = vk::ImportSemaphoreWin32HandleInfoKHR::default()
                .handle(handle.handle() as vk::HANDLE)
                .handle_type(type_bit)
                .semaphore(semaphore.handle());
            api.import_semaphore_win32_handle_khr(device, &info).throw_on_error("vkImportSemaphoreWin32HandleKHR")
        };
        if let Err(error) = imported {
            semaphore.dispose();
            panic!("{error}");
        }
        Rc::new(ImportedSemaphore { context: self.context.clone(), pool: self.pool.clone(), sem: RefCell::new(Some(semaphore)) })
    }
}

struct ImportedSemaphore {
    context: Rc<dyn IVulkanPlatformGraphicsContext>,
    pool: Rc<VulkanCommandBufferPool>,
    sem: RefCell<Option<VulkanSemaphore>>,
}

impl ImportedSemaphore {
    fn submit_semaphore(&self, wait: bool, signal: bool) -> Result<(), VulkanError> {
        let sem = self.sem.borrow();
        let buf = self.pool.create_command_buffer()?;
        buf.begin_recording()?;
        self.context.device_api().cmd_pipeline_barrier(
            buf.handle(),
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::DependencyFlags::empty(),
            &[],
        );
        buf.end_recording()?;
        let wait: Vec<&VulkanSemaphore> = sem.iter().filter(|_| wait).collect();
        let signal: Vec<&VulkanSemaphore> = sem.iter().filter(|_| signal).collect();
        buf.submit_with(&wait, &[vk::PipelineStageFlags::ALL_GRAPHICS], &signal, None)
    }
}

impl IVulkanExternalSemaphore for ImportedSemaphore {
    fn handle(&self) -> u64 {
        match self.sem.borrow().as_ref() {
            Some(sem) => vk::Handle::as_raw(sem.handle()),
            None => panic!("Cannot access a disposed object.\nObject name: 'ImportedSemaphore'."),
        }
    }

    fn submit_wait_semaphore(&self) {
        self.submit_semaphore(true, false).unwrap_or_else(|error| panic!("{error}"));
    }

    fn submit_signal_semaphore(&self) {
        self.submit_semaphore(false, true).unwrap_or_else(|error| panic!("{error}"));
    }

    fn dispose(&self) {
        if let Some(sem) = self.sem.borrow_mut().take() {
            sem.dispose();
        }
    }
}

struct ImportedImage {
    base: VulkanImageBase,
}

impl ImportedImage {
    fn new(
        context: Rc<dyn IVulkanPlatformGraphicsContext>,
        command_buffer_pool: Rc<VulkanCommandBufferPool>,
        import_handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Result<Self, VulkanError> {
        let base = VulkanImageBase::new(
            context.clone(),
            command_buffer_pool,
            image_format(properties.format),
            PixelSize::new(properties.width, properties.height),
            1,
        );
        let type_bit = memory_handle_type(import_handle.handle_descriptor())
            .unwrap_or_else(|| panic!("Specified method is not supported."));
        let mut external_alloc = vk::ExternalMemoryImageCreateInfo::default().handle_types(type_bit);

        let create_memory = |image: vk::Image, size: u64, memory_type_bits: u32| {
            let handle = &import_handle;

            if properties.memory_offset != 0 || properties.memory_size != size {
                return Err(VulkanError::new("Invalid memory size"));
            }

            let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);

            let is_posix_handle = handle.handle_descriptor()
                == Some(KnownPlatformGraphicsExternalImageHandleTypes::VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR);
            let mut win32_info = vk::ImportMemoryWin32HandleInfoKHR::default()
                .handle(handle.handle() as vk::HANDLE)
                .handle_type(type_bit);
            let mut posix_info = vk::ImportMemoryFdInfoKHR::default()
                .handle_type(type_bit)
                .fd(if is_posix_handle { handle.handle() as i32 } else { 0 });

            let memory_allocate_info = vk::MemoryAllocateInfo::default()
                .allocation_size(size)
                .memory_type_index(VulkanMemoryHelper::find_suitable_memory_type_index(
                    &*context,
                    memory_type_bits,
                    vk::MemoryPropertyFlags::DEVICE_LOCAL,
                ) as u32)
                .push_next(&mut dedicated);
            let memory_allocate_info = if is_posix_handle {
                memory_allocate_info.push_next(&mut posix_info)
            } else {
                memory_allocate_info.push_next(&mut win32_info)
            };

            context
                .device_api()
                .allocate_memory(context.device_handle(), &memory_allocate_info)
                .throw_on_error("vkAllocateMemory")
        };

        base.initialize(Some(&mut external_alloc), Some(&create_memory))?;
        base.set_current_layout(
            properties
                .vulkan_properties
                .map_or(vk::ImageLayout::UNDEFINED, |vulkan_properties| vk::ImageLayout::from_raw(vulkan_properties.layout)),
        );
        base.transition_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL, vk::AccessFlags::TRANSFER_READ)?;
        Ok(Self { base })
    }
}

impl IVulkanExternalImage for ImportedImage {
    fn info(&self) -> VulkanImageInfo {
        self.base.image_info()
    }

    fn dispose(&self) {
        self.base.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_required_extensions_are_those_of_the_reference() {
        assert_eq!(
            VulkanExternalObjectsFeature::REQUIRED_INSTANCE_EXTENSIONS,
            [
                "VK_KHR_get_physical_device_properties2",
                "VK_KHR_external_memory_capabilities",
                "VK_KHR_external_semaphore_capabilities"
            ]
        );
        assert_eq!(
            VulkanExternalObjectsFeature::required_device_extensions_for(false),
            [
                "VK_KHR_external_memory",
                "VK_KHR_external_semaphore",
                "VK_KHR_dedicated_allocation",
                "VK_KHR_external_semaphore_fd",
                "VK_KHR_external_memory_fd"
            ]
        );
        assert_eq!(
            VulkanExternalObjectsFeature::required_device_extensions_for(true),
            [
                "VK_KHR_external_memory",
                "VK_KHR_external_semaphore",
                "VK_KHR_dedicated_allocation",
                "VK_KHR_external_semaphore_win32",
                "VK_KHR_external_memory_win32"
            ]
        );
        assert_eq!(
            VulkanExternalObjectsFeature::required_device_extensions(),
            VulkanExternalObjectsFeature::required_device_extensions_for(cfg!(windows))
        );
    }

    #[test]
    fn handles_are_file_descriptors_or_windows_handles() {
        let (images, semaphores) = VulkanExternalObjectsFeature::supported_types_for(false);
        assert_eq!(images, ["VulkanOpaquePosixFileDescriptor"]);
        assert_eq!(semaphores, ["VulkanOpaquePosixFileDescriptor"]);
        let (images, semaphores) = VulkanExternalObjectsFeature::supported_types_for(true);
        assert_eq!(images, ["VulkanOpaqueNtHandle", "VulkanOpaqueKmtHandle"]);
        assert_eq!(semaphores, ["VulkanOpaqueNtHandle", "VulkanOpaqueKmtHandle"]);
    }

    #[test]
    fn a_handle_kind_maps_to_the_handle_type_of_vulkan() {
        use vk::ExternalMemoryHandleTypeFlags as Memory;
        use vk::ExternalSemaphoreHandleTypeFlags as Semaphore;
        assert_eq!(memory_handle_type(Some("VulkanOpaquePosixFileDescriptor")), Some(Memory::OPAQUE_FD));
        assert_eq!(memory_handle_type(Some("VulkanOpaqueKmtHandle")), Some(Memory::OPAQUE_WIN32_KMT));
        assert_eq!(memory_handle_type(Some("VulkanOpaqueNtHandle")), Some(Memory::OPAQUE_WIN32));
        assert_eq!(memory_handle_type(Some("IOSurfaceRef")), None);
        assert_eq!(memory_handle_type(None), None);
        assert_eq!(semaphore_handle_type(Some("VulkanOpaquePosixFileDescriptor")), Some(Semaphore::OPAQUE_FD));
        assert_eq!(semaphore_handle_type(Some("VulkanOpaqueKmtHandle")), Some(Semaphore::OPAQUE_WIN32_KMT));
        assert_eq!(semaphore_handle_type(Some("VulkanOpaqueNtHandle")), Some(Semaphore::OPAQUE_WIN32));
        assert_eq!(semaphore_handle_type(Some("MetalSharedEvent")), None);
        assert_eq!(semaphore_handle_type(None), None);
    }

    #[test]
    fn an_image_format_maps_to_the_format_of_vulkan() {
        assert_eq!(image_format(PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm), vk::Format::B8G8R8A8_UNORM);
        assert_eq!(image_format(PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm), vk::Format::R8G8B8A8_UNORM);
    }

    #[test]
    fn an_identifier_of_zeros_is_no_identifier() {
        assert_eq!(non_zero(&[0; 8]), None);
        assert_eq!(non_zero(&[0, 0, 3, 0]), Some(vec![0, 0, 3, 0]));
    }

    #[test]
    fn a_handle_kind_is_supported_when_it_is_listed() {
        let types = vec!["VulkanOpaquePosixFileDescriptor".to_string()];
        assert!(supports(&types, Some("VulkanOpaquePosixFileDescriptor")));
        assert!(!supports(&types, Some("VulkanOpaqueNtHandle")));
        assert!(!supports(&types, None));
    }
}
