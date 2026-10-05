/// The description of a GPU image to import.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PlatformGraphicsExternalImageProperties {
    pub width: i32,
    pub height: i32,
    pub format: PlatformGraphicsExternalImageFormat,
    pub memory_size: u64,
    pub memory_offset: u64,
    pub top_left_origin: bool,
    /// Vulkan-specific properties of the imported image, ignored by other backends.
    pub vulkan_properties: Option<PlatformGraphicsExternalImageVulkanProperties>,
    /// dma-buf-specific properties of the imported image.
    pub dma_buf_properties: Option<PlatformGraphicsExternalImageDmaBufProperties>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PlatformGraphicsExternalImageVulkanProperties {
    /// The VkImageLayout the underlying memory is currently in.
    pub layout: i32,
}

/// Describes the DRM layout of a dma-buf image.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PlatformGraphicsExternalImageDmaBufProperties {
    /// The DRM_FORMAT_* fourcc of the buffer.
    pub drm_format: u32,
    /// The DRM_FORMAT_MOD_* layout modifier of the buffer, or [`DRM_MODIFIER_INVALID`](Self::DRM_MODIFIER_INVALID) for the implicit layout.
    pub drm_modifier: u64,
    /// The number of memory planes. When zero, a single plane described by the handle itself and
    /// [`PlatformGraphicsExternalImageProperties::memory_offset`] is assumed.
    pub plane_count: i32,
    /// Per-plane dma-buf file descriptors. When `None`, the handle's file descriptor is used for every plane.
    pub plane_fds: Option<Vec<i32>>,
    /// Per-plane row pitches in bytes.
    pub plane_strides: Option<Vec<u32>>,
    /// Per-plane byte offsets into the corresponding file descriptor.
    pub plane_offsets: Option<Vec<u32>>,
}

impl PlatformGraphicsExternalImageDmaBufProperties {
    /// The DRM_FORMAT_MOD_INVALID modifier.
    pub const DRM_MODIFIER_INVALID: u64 = 0x00ff_ffff_ffff_ffff;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PlatformGraphicsExternalImageFormat {
    #[default]
    R8G8B8A8UNorm,
    B8G8R8A8UNorm,
}

/// A DRM fourcc format + modifier pair the GPU backend can import as a
/// [`KnownPlatformGraphicsExternalImageHandleTypes::DMA_BUF_FILE_DESCRIPTOR`] image.
/// `format` is a DRM_FORMAT_* fourcc; `modifier` is a
/// DRM_FORMAT_MOD_* layout ([`PlatformGraphicsExternalImageDmaBufProperties::DRM_MODIFIER_INVALID`]
/// meaning the implicit, driver-chosen layout).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlatformGraphicsDrmFormat {
    pub format: u32,
    pub modifier: u64,
}

impl PlatformGraphicsDrmFormat {
    pub fn new(format: u32, modifier: u64) -> Self {
        Self { format, modifier }
    }
}

/// Describes various GPU memory handle types that are currently supported by the graphics backends
pub struct KnownPlatformGraphicsExternalImageHandleTypes;

impl KnownPlatformGraphicsExternalImageHandleTypes {
    /// An DXGI global shared handle returned by IDXGIResource::GetSharedHandle D3D11_RESOURCE_MISC_SHARED or D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX flag.
    /// The handle does not own the reference to the underlying video memory, so the provider should make sure that the resource is valid until
    /// the handle has been successfully imported
    pub const D3D11_TEXTURE_GLOBAL_SHARED_HANDLE: &'static str = "D3D11TextureGlobalSharedHandle";
    /// A DXGI NT handle returned by IDXGIResource1::CreateSharedHandle for a texture created with D3D11_RESOURCE_MISC_SHARED_NTHANDLE or flag
    pub const D3D11_TEXTURE_NT_HANDLE: &'static str = "D3D11TextureNtHandle";
    /// A POSIX file descriptor that's exported by Vulkan using VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_FD_BIT or in a compatible way
    pub const VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR: &'static str = "VulkanOpaquePosixFileDescriptor";
    /// A NT handle that's been exported by Vulkan using VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32_BIT or in a compatible way
    pub const VULKAN_OPAQUE_NT_HANDLE: &'static str = "VulkanOpaqueNtHandle";
    // A global shared handle that's been exported by Vulkan using VK_EXTERNAL_MEMORY_HANDLE_TYPE_OPAQUE_WIN32_KMT_BIT or in a compatible way
    pub const VULKAN_OPAQUE_KMT_HANDLE: &'static str = "VulkanOpaqueKmtHandle";
    /// A reference to IOSurface
    pub const IO_SURFACE_REF: &'static str = "IOSurfaceRef";
    /// A Linux dma-buf file descriptor, imported via EGL_LINUX_DMA_BUF_EXT or VK_EXTERNAL_MEMORY_HANDLE_TYPE_DMA_BUF_BIT_EXT.
    pub const DMA_BUF_FILE_DESCRIPTOR: &'static str = "DmaBufFileDescriptor";
}

/// Describes various GPU semaphore handle types that are currently supported by the graphics backends
pub struct KnownPlatformGraphicsExternalSemaphoreHandleTypes;

impl KnownPlatformGraphicsExternalSemaphoreHandleTypes {
    /// A POSIX file descriptor that's been exported by Vulkan using VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_FD_BIT or in a compatible way
    pub const VULKAN_OPAQUE_POSIX_FILE_DESCRIPTOR: &'static str = "VulkanOpaquePosixFileDescriptor";
    /// A NT handle that's been exported by Vulkan using VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_WIN32_BIT or in a compatible way
    pub const VULKAN_OPAQUE_NT_HANDLE: &'static str = "VulkanOpaqueNtHandle";
    // A global shared handle that's been exported by Vulkan using VK_EXTERNAL_SEMAPHORE_HANDLE_TYPE_OPAQUE_WIN32_KMT_BIT or in a compatible way
    pub const VULKAN_OPAQUE_KMT_HANDLE: &'static str = "VulkanOpaqueKmtHandle";
    /// A DXGI NT handle returned by ID3D12Device::CreateSharedHandle or ID3D11Fence::CreateSharedHandle
    pub const DIRECT3D12_FENCE_NT_HANDLE: &'static str = "Direct3D12FenceNtHandle";
    /// A pointer to MTLSharedEvent object
    pub const METAL_SHARED_EVENT: &'static str = "MetalSharedEvent";
}
