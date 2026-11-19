//! The shared frames of videos and cameras on the device of the GPU modes over the context of
//! OpenGL of the platform (`gl_context_gpu.rs`: Linux and Windows without a hardware Vulkan or
//! Direct3D 12 adapter), #575.
//!
//! A producer of the platform that draws its frames with the context hands each out as a texture
//! of the context (`SHARED_GL_TEXTURE`, its owner a [`SharedFrameTexture`] that the lease of the
//! frame carries), as it does for the Skia backend (`gl_skia_external_objects_feature.rs` there).
//! The device of the GPU modes is `wgpu` over that very context, so the texture is wrapped as a
//! texture of the device as it is (`wgpu-hal`, `gles::Device::texture_from_raw`, borrowed: the
//! storage stays the producer's) and the frame becomes a [`VelloSharedFrameBitmap`], converted on
//! the device by one pass and its lease given back once the device has done that pass, as on
//! macOS (`external_objects.rs`). Nothing goes through memory.
//!
//! The order on the GPU is the order of the commands of the one context, so nothing is signalled
//! between the producer and the device (the synchronization is automatic).
//!
//! This file has `unsafe` for the raw texture of the context.

use crate::gpu::{VelloDeviceTexture, VelloSharedFrameBitmap, VelloTextureAlpha, VelloWgpuDevice};
use ferroui_base::platform::{
    IExternalObjectsRenderInterfaceContextFeature, IPlatformHandle, IPlatformRenderInterfaceImportedImage,
    IPlatformRenderInterfaceImportedObject, IPlatformRenderInterfaceImportedSemaphore,
    KnownPlatformGraphicsExternalImageHandleTypes, PlatformGraphicsExternalImageFormat,
    PlatformGraphicsExternalImageProperties, SharedBitmapImpl,
};
use ferroui_base::rendering::composition::frames::{
    DeviceIdentity, FrameAlpha, FrameColor, FrameError, FrameFormat, FrameLease,
};
use ferroui_base::rendering::composition::{
    CompositionGpuImportedImageSynchronizationCapabilities, ICompositionImportableSharedGpuContextImage,
};
use ferroui_opengl::SharedFrameTexture;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;
use wgpu::hal;

/// The largest frame the backend imports, in pixels on a side (as on macOS).
const MAX_FRAME_SIDE: u32 = 8192;

/// The external objects feature of the device of the GPU modes over the context of OpenGL of the
/// platform: the frames of the context's own producers.
pub struct VelloGlExternalObjectsFeature {
    device: Arc<VelloWgpuDevice>,
    /// The context the frames are textures of, and the generation of its resources: a frame of
    /// another context, or made before the context was recreated, is refused.
    context_id: u64,
    generation: u64,
}

impl VelloGlExternalObjectsFeature {
    /// The feature of `device`, made over the context `context_id` of the given generation (what
    /// `IGlContext::shared_frame_context_id` and `shared_frame_context_generation` say).
    pub(crate) fn new(device: Arc<VelloWgpuDevice>, context_id: u64, generation: u64) -> Self {
        Self { device, context_id, generation }
    }
}

/// A texture of the context a producer draws its frames into, wrapped as a texture of the device.
pub struct VelloGlImportedImage {
    device: Arc<VelloWgpuDevice>,
    context_id: u64,
    generation: u64,
    texture_id: u32,
    properties: PlatformGraphicsExternalImageProperties,
    /// The wrapped texture, or why it cannot be wrapped.
    texture: Result<Arc<VelloDeviceTexture>, String>,
}

impl IPlatformRenderInterfaceImportedObject for VelloGlImportedImage {
    /// The texture is the producer's: nothing is deleted.
    fn dispose(&self) {}
}

/// The texture `name` of the context the device was made over as a texture of the device, in
/// RGBA8, borrowed (it is not deleted when the wrapped texture is dropped).
fn wrap_gl_texture(device: &Arc<VelloWgpuDevice>, name: u32, width: u32, height: u32) -> Result<wgpu::Texture, String> {
    let name = NonZeroU32::new(name).ok_or("the frame has no texture")?;
    let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let hal_descriptor = hal::TextureDescriptor {
        label: Some("FerroUI Vello shared frame"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUses::RESOURCE,
        memory_flags: hal::MemoryFlags::empty(),
        view_formats: Vec::new(),
    };
    // SAFETY: the device is `wgpu` over the context the texture belongs to (the feature is made
    // for that context alone, and the frame says which context it is of); the texture is a
    // two-dimensional RGBA8 texture of one level of the size of the frame, which the producer
    // keeps alive while the lease of the frame is held. The drop callback leaves it to its owner.
    let hal_texture = unsafe {
        let hal_device =
            device.device().as_hal::<hal::api::Gles>().ok_or("the device of the GPU modes is not one of OpenGL")?;
        hal_device.texture_from_raw(name, &hal_descriptor, Some(Box::new(|| {})))
    };
    let descriptor = wgpu::TextureDescriptor {
        label: Some("FerroUI Vello shared frame"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    // SAFETY: the texture is one of this device's context and matches the descriptor; its content
    // is the producer's frame, complete when it was handed over: it is read as initialized.
    Ok(unsafe {
        device.device().create_texture_from_hal::<hal::api::Gles>(hal_texture, &descriptor, wgpu::TextureUses::RESOURCE)
    })
}

impl IPlatformRenderInterfaceImportedImage for VelloGlImportedImage {
    fn acquire_shared_bitmap(&self, frame: FrameLease) -> Result<Arc<SharedBitmapImpl>, FrameError> {
        let owner = frame.resource::<SharedFrameTexture>().ok_or(FrameError::MissingImporter)?;
        let m = frame.metadata();
        m.validate(MAX_FRAME_SIDE)?;
        if self.device.is_lost()
            || !frame.is_current()
            || m.device_generation != self.generation
            || owner.generation() != self.generation
        {
            return Err(FrameError::StaleGeneration);
        }
        if m.device != Some(DeviceIdentity::Context(self.context_id))
            || owner.context_id() != self.context_id
            || owner.texture() != self.texture_id
        {
            return Err(FrameError::DeviceMismatch);
        }
        let texture = self.texture.as_ref().map_err(|_| FrameError::MissingImporter)?;
        if m.width as i32 != self.properties.width
            || m.height as i32 != self.properties.height
            || m.width != texture.width()
            || m.height != texture.height()
        {
            return Err(FrameError::InvalidDimensions);
        }
        let format = self.properties.format;
        if m.format != FrameFormat::Rgba8 || format != PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm {
            return Err(FrameError::UnsupportedFormat);
        }
        match m.color {
            FrameColor::Srgb => {}
            // Video: the curve of the reference display, for opaque frames.
            FrameColor::Rec709Full if m.alpha == FrameAlpha::Opaque => {}
            _ => return Err(FrameError::UnsupportedColor),
        }
        if m.alpha == FrameAlpha::Straight {
            return Err(FrameError::UnsupportedFormat);
        }
        frame.counters().gpu_import();
        Ok(Arc::new(VelloSharedFrameBitmap::new(texture.clone(), frame)))
    }

    /// # Panics
    /// Always: the frames of the context are shared, not copied.
    fn snapshot_with_keyed_mutex(&self, _acquire_index: u32, _release_index: u32) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: the frames of the context are shared, not copied.
    fn snapshot_with_semaphores(
        &self,
        _wait_for_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _signal_semaphore: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
    ) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: the frames of the context are shared, not copied.
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
    /// Always: the frames of the context are shared ([`acquire_shared_bitmap`](Self::acquire_shared_bitmap)).
    fn snapshot_with_automatic_sync(&self) -> Arc<SharedBitmapImpl> {
        panic!("Specified method is not supported.");
    }
}

impl IExternalObjectsRenderInterfaceContextFeature for VelloGlExternalObjectsFeature {
    fn shared_context_id(&self) -> Option<u64> {
        Some(self.context_id)
    }

    fn shared_context_generation(&self) -> u64 {
        self.generation
    }

    fn supported_image_handle_types(&self) -> Vec<String> {
        vec![KnownPlatformGraphicsExternalImageHandleTypes::SHARED_GL_TEXTURE.into()]
    }

    fn supported_semaphore_types(&self) -> Vec<String> {
        Vec::new()
    }

    /// # Panics
    /// Panics for a handle that is not a texture of the context (`SHARED_GL_TEXTURE`), as the Skia
    /// backend does for a handle it cannot import.
    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        if handle.handle_descriptor() != Some(KnownPlatformGraphicsExternalImageHandleTypes::SHARED_GL_TEXTURE) {
            panic!("Importing this platform handle is not supported");
        }
        let texture_id = u32::try_from(handle.handle()).ok().filter(|id| *id != 0).expect("Invalid frame texture name");
        let (width, height) = (properties.width.max(0) as u32, properties.height.max(0) as u32);
        let texture = wrap_gl_texture(&self.device, texture_id, width, height)
            .map(|texture| VelloDeviceTexture::from_external(&self.device, texture, VelloTextureAlpha::Premultiplied));
        Rc::new(VelloGlImportedImage {
            device: self.device.clone(),
            context_id: self.context_id,
            generation: self.generation,
            texture_id,
            properties,
            texture,
        })
    }

    /// # Panics
    /// Always: the frames are imported by their texture.
    fn import_shared_image(
        &self,
        _image: Arc<dyn ICompositionImportableSharedGpuContextImage>,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: the order of the commands of the context needs no semaphore.
    fn import_semaphore(&self, _handle: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore> {
        panic!("Specified method is not supported.");
    }

    fn get_synchronization_capabilities(
        &self,
        image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        if image_handle_type == KnownPlatformGraphicsExternalImageHandleTypes::SHARED_GL_TEXTURE {
            return CompositionGpuImportedImageSynchronizationCapabilities::AUTOMATIC;
        }
        CompositionGpuImportedImageSynchronizationCapabilities::default()
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        None
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        None
    }
}
