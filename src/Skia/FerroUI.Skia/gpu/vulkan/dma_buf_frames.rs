//! The frames of a GStreamer session on Ganesh over Vulkan (#575, #608): DMA-BUFs imported by
//! `ferroui_vulkan::dma_buf`, off by default behind `FERROUI_VULKAN_DMA_BUF=1`.
//!
//! An XRGB or XBGR frame is borrowed as one Ganesh image. An NV12 frame is two borrowed plane
//! textures, which Ganesh converts with its YUVA pipeline when the image is drawn. Each comes from
//! the foreign queue family, which Ganesh acquires it from on first use. A frame that cannot be
//! imported is refused; nothing is read on the CPU.

use crate::gpu::ganesh::GaneshGrContext;
use crate::gpu::ISkiaGrContext;
use crate::leased_gpu_bitmap::LeasedGpuBitmap;
use ferroui_base::platform::{
    IPlatformRenderInterfaceImportedImage, IPlatformRenderInterfaceImportedObject,
    IPlatformRenderInterfaceImportedSemaphore, PlatformGraphicsExternalImageProperties, SharedBitmapImpl,
};
use ferroui_base::rendering::composition::frames::{
    DeviceIdentity, FrameAlpha, FrameColor, FrameError, FrameFormat, FrameLease,
};
use ferroui_vulkan::ash::vk::{self as ash_vk, Handle};
use ferroui_vulkan::dma_buf::{VulkanDmaBufImporter, VulkanDmaBufPlane, VulkanDmaBufPlanes, DRM_NV12, DRM_XB24};
use skia_safe::gpu::{backend_textures, images, vk, BackendTexture, Protected, SurfaceOrigin, YUVABackendTextures};
use skia_safe::{yuva_info, AlphaType, ColorType, Image, YUVAInfo, YUVColorSpace};
use std::rc::Rc;
use std::sync::Arc;

/// The largest frame the backend imports, in pixels on a side (as on the other routes).
const MAX_FRAME_SIDE: u32 = 8192;

/// A plane as a backend texture of Ganesh, in the foreign queue family.
fn plane_texture(plane: &VulkanDmaBufPlane) -> Option<BackendTexture> {
    let format = match plane.format() {
        ash_vk::Format::B8G8R8A8_UNORM => vk::Format::B8G8R8A8_UNORM,
        ash_vk::Format::R8G8B8A8_UNORM => vk::Format::R8G8B8A8_UNORM,
        ash_vk::Format::R8_UNORM => vk::Format::R8_UNORM,
        ash_vk::Format::R8G8_UNORM => vk::Format::R8G8_UNORM,
        _ => return None,
    };
    let (width, height) = plane.size();
    // SAFETY: the image and its memory are objects of the device of the context. The planes are
    // held by the bitmap and destroyed only after the queue has done what reads them.
    unsafe {
        let alloc = vk::Alloc::from_device_memory(
            plane.memory().as_raw() as usize as _,
            0,
            plane.allocation_size(),
            vk::AllocFlag::empty(),
        );
        let mut info = vk::ImageInfo::new(
            plane.image().as_raw() as usize as _,
            alloc,
            vk::ImageTiling::DRM_FORMAT_MODIFIER_EXT,
            vk::ImageLayout::GENERAL,
            format,
            1,
            ash_vk::QUEUE_FAMILY_FOREIGN_EXT,
            None,
            Protected::No,
            None,
        );
        info.image_usage_flags = ash_vk::ImageUsageFlags::SAMPLED.as_raw();
        info.sample_count = 1;
        Some(backend_textures::make_vk((width as i32, height as i32), &info, ""))
    }
}

/// The Ganesh image of an imported buffer.
fn frame_image(gr: &GaneshGrContext, planes: &VulkanDmaBufPlanes, m_color: FrameColor) -> Option<Image> {
    let textures: Option<Vec<BackendTexture>> = planes.planes().iter().map(plane_texture).collect();
    let textures = textures?;
    let (width, height) = planes.planes().first()?.size();
    if let [single] = textures.as_slice() {
        let color_type = match planes.planes()[0].format() {
            ash_vk::Format::B8G8R8A8_UNORM => ColorType::BGRA8888,
            _ => ColorType::RGBA8888,
        };
        return gr.with_context(|context| {
            images::borrow_texture_from(context, single, SurfaceOrigin::TopLeft, color_type, AlphaType::Opaque, None)
        });
    }
    let space = match m_color {
        FrameColor::Rec709Limited => YUVColorSpace::Rec709_Limited,
        _ => YUVColorSpace::Rec709_Full,
    };
    let info = YUVAInfo::new(
        (width as i32, height as i32),
        yuva_info::PlaneConfig::Y_UV,
        yuva_info::Subsampling::S420,
        space,
        None,
        (yuva_info::Siting::Centered, yuva_info::Siting::Centered),
    )?;
    let yuva = YUVABackendTextures::new(&info, &textures, SurfaceOrigin::TopLeft)?;
    gr.with_context(|context| images::texture_from_yuva_textures(context, &yuva, None))
}

/// A DMA-BUF frame imported into the device of a Ganesh context.
pub(super) struct DmaBufImage {
    gr: Rc<GaneshGrContext>,
    importer: Rc<VulkanDmaBufImporter>,
    generation: u64,
    planes: Result<Arc<VulkanDmaBufPlanes>, String>,
    drm_format: u32,
    properties: PlatformGraphicsExternalImageProperties,
}

impl DmaBufImage {
    pub(super) fn new(
        gr: Rc<GaneshGrContext>,
        importer: Rc<VulkanDmaBufImporter>,
        generation: u64,
        handle: i32,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Self {
        importer.collect(gr.flush_count());
        let (width, height) = (properties.width.max(0) as u32, properties.height.max(0) as u32);
        let dma = properties.dma_buf_properties.clone();
        let drm_format = dma.as_ref().map_or(0, |dma| dma.drm_format);
        let planes = match &dma {
            Some(dma) => {
                let count = dma.plane_count.max(0) as usize;
                let at = |values: &Option<Vec<u32>>, plane: usize| values.as_ref().and_then(|v| v.get(plane).copied());
                let layouts: Option<Vec<(i32, u32, u32)>> = (0..count)
                    .map(|plane| {
                        let fd = dma.plane_fds.as_ref().and_then(|fds| fds.get(plane).copied()).unwrap_or(handle);
                        Some((fd, at(&dma.plane_offsets, plane)?, at(&dma.plane_strides, plane)?))
                    })
                    .collect();
                match layouts {
                    Some(layouts) => importer.import(dma.drm_format, dma.drm_modifier, (width, height), &layouts),
                    None => Err("the buffer has no plane layout".to_owned()),
                }
            }
            None => Err("the handle has no DMA-BUF layout".to_owned()),
        };
        Self { gr, importer, generation, planes, drm_format, properties }
    }
}

impl IPlatformRenderInterfaceImportedObject for DmaBufImage {
    /// The planes go with their last owner, once the queue has done what reads them.
    fn dispose(&self) {}
}

impl IPlatformRenderInterfaceImportedImage for DmaBufImage {
    fn acquire_shared_bitmap(&self, frame: FrameLease) -> Result<Arc<SharedBitmapImpl>, FrameError> {
        let m = frame.metadata();
        m.validate(MAX_FRAME_SIDE)?;
        if !frame.is_current() || m.device_generation != self.generation {
            return Err(FrameError::StaleGeneration);
        }
        if m.device.as_ref() != Some(&DeviceIdentity::Uuid(self.importer.device_uuid().to_vec())) {
            return Err(FrameError::DeviceMismatch);
        }
        if m.width as i32 != self.properties.width || m.height as i32 != self.properties.height || !m.top_left_origin {
            return Err(FrameError::InvalidDimensions);
        }
        let format_matches = match m.format {
            FrameFormat::Nv12 => self.drm_format == DRM_NV12,
            FrameFormat::Rgba8 => self.drm_format == DRM_XB24,
            FrameFormat::Bgra8 => self.drm_format == ferroui_vulkan::dma_buf::DRM_XR24,
        };
        if !format_matches || m.alpha != FrameAlpha::Opaque {
            return Err(FrameError::UnsupportedFormat);
        }
        let planes = self.planes.as_ref().map_err(|_| FrameError::MissingImporter)?;
        self.importer.collect(self.gr.flush_count());
        let image = frame_image(&self.gr, planes, m.color).ok_or(FrameError::MissingImporter)?;
        let context = self.gr.frame_context_id().ok_or(FrameError::MissingSynchronization)?;
        // After Ganesh's YUVA conversion the values are RGB on the curve of the video.
        let color = if m.format == FrameFormat::Nv12 { FrameColor::Rec709Full } else { m.color };
        frame.counters().gpu_import();
        Ok(Arc::new(LeasedGpuBitmap::with_owner(image, frame, context, color, planes.clone())))
    }

    /// # Panics
    /// Always: frames are shared, not copied.
    fn snapshot_with_keyed_mutex(&self, _acquire_index: u32, _release_index: u32) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: frames are shared, not copied.
    fn snapshot_with_semaphores(
        &self,
        _wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
    ) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: frames are shared, not copied.
    fn snapshot_with_timeline_semaphores(
        &self,
        _wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _wait_for_value: u64,
        _signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _signal_value: u64,
    ) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: frames are shared ([`acquire_shared_bitmap`](Self::acquire_shared_bitmap)).
    fn snapshot_with_automatic_sync(&self) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }
}
