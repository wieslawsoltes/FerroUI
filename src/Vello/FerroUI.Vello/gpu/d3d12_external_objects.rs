//! The shared frames of videos and cameras on the Direct3D 12 device of the GPU modes on Windows
//! (#575).
//!
//! **Behind a switch.** The environment variable `FERROUI_VELLO_NT_HANDLE_IMPORT=1` turns this
//! route on, until a run in the Windows virtual machine has proved it. Without it the device reports
//! no import, as before. The device itself is made the same way in both cases.
//!
//! The producer copies each frame on its Direct3D 11 device into a texture shared by an NT handle
//! and hands it over once that copy has completed. The texture is opened here with
//! `ID3D12Device::OpenSharedHandle`, wrapped as a texture of the device and becomes a
//! [`VelloSharedFrameBitmap`], converted on the device by one pass with its lease given back when
//! the queue has done that pass, as on macOS. Nothing goes through memory. A texture that cannot
//! be opened refuses the frame; there is no CPU path.
//!
//! This file has `unsafe` for the raw objects of Direct3D 12.

use crate::gpu::{VelloDeviceTexture, VelloSharedFrameBitmap, VelloTextureAlpha, VelloWgpuDevice};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::{
    IExternalObjectsRenderInterfaceContextFeature, IPlatformHandle, IPlatformRenderInterfaceImportedImage,
    IPlatformRenderInterfaceImportedObject, IPlatformRenderInterfaceImportedSemaphore,
    KnownPlatformGraphicsExternalImageHandleTypes as HandleTypes, PlatformGraphicsExternalImageFormat,
    PlatformGraphicsExternalImageProperties, SharedBitmapImpl,
};
use ferroui_base::rendering::composition::frames::{
    DeviceIdentity, FrameAlpha, FrameColor, FrameError, FrameFormat, FrameLease,
};
use ferroui_base::rendering::composition::{
    CompositionGpuImportedImageSynchronizationCapabilities, ICompositionImportableSharedGpuContextImage,
};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use wgpu::hal;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Graphics::Direct3D12::ID3D12Resource;

/// The largest frame the backend imports, in pixels on a side (as on the other routes).
const MAX_FRAME_SIDE: u32 = 8192;

/// Generations of the feature: a frame made for a device that was replaced is refused.
static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Reports once that a texture was not opened: its frames are refused.
fn report_import_failure(error: &str) {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if REPORTED.swap(true, Ordering::Relaxed) {
        return;
    }
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
        logger.log_with_values(None, "The Vello backend could not open a shared frame: {Error}", &[&error]);
    }
}

/// The external objects feature of a Direct3D 12 device of the GPU modes.
pub struct VelloD3d12ExternalObjectsFeature {
    device: Arc<VelloWgpuDevice>,
    generation: u64,
    luid: Vec<u8>,
}

impl VelloD3d12ExternalObjectsFeature {
    /// The feature of `device`, when the route is turned on and the device is one of Direct3D 12.
    pub(crate) fn new(device: Arc<VelloWgpuDevice>) -> Option<Self> {
        if !std::env::var("FERROUI_VELLO_NT_HANDLE_IMPORT").is_ok_and(|value| value == "1") {
            return None;
        }
        // SAFETY: the raw device is only asked for its adapter.
        let luid = unsafe {
            let hal_device = device.device().as_hal::<hal::api::Dx12>()?;
            let luid = hal_device.raw_device().GetAdapterLuid();
            let mut bytes = luid.LowPart.to_le_bytes().to_vec();
            bytes.extend_from_slice(&luid.HighPart.to_le_bytes());
            bytes
        };
        Some(Self { device, generation: NEXT_GENERATION.fetch_add(1, Ordering::Relaxed), luid })
    }

    /// The texture of the NT handle `handle` as a texture of the device.
    fn open(&self, handle: isize, width: u32, height: u32) -> Result<Arc<VelloDeviceTexture>, String> {
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let format = wgpu::TextureFormat::Bgra8Unorm;
        // SAFETY: `handle` is an NT handle of a two-dimensional BGRA8 texture of one level of this
        // size on the adapter of this device, which the lease of the frame keeps open for the call.
        // The opened resource holds its own reference, and the producer's copy has completed.
        unsafe {
            let resource = {
                let hal_device = self
                    .device
                    .device()
                    .as_hal::<hal::api::Dx12>()
                    .ok_or("the device of the GPU modes is not one of Direct3D 12")?;
                let mut resource: Option<ID3D12Resource> = None;
                hal_device
                    .raw_device()
                    .OpenSharedHandle(HANDLE(handle as *mut core::ffi::c_void), &mut resource)
                    .map_err(|error| format!("OpenSharedHandle failed: {error}"))?;
                resource.ok_or("OpenSharedHandle gave no resource")?
            };
            let hal_texture =
                hal::dx12::Device::texture_from_raw(resource, format, wgpu::TextureDimension::D2, size, 1, 1);
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
            // A shared resource starts in the common state, which is `PRESENT` here.
            let texture = self.device.device().create_texture_from_hal::<hal::api::Dx12>(
                hal_texture,
                &descriptor,
                wgpu::TextureUses::PRESENT,
            );
            Ok(VelloDeviceTexture::from_external(&self.device, texture, VelloTextureAlpha::Premultiplied))
        }
    }
}

/// A shared texture of a frame, opened on the device.
pub struct VelloD3d12ImportedImage {
    device: Arc<VelloWgpuDevice>,
    generation: u64,
    luid: Vec<u8>,
    /// The wrapped texture, or why it could not be opened.
    texture: Result<Arc<VelloDeviceTexture>, String>,
    format: PlatformGraphicsExternalImageFormat,
}

impl IPlatformRenderInterfaceImportedObject for VelloD3d12ImportedImage {
    /// The storage is the producer's, and the opened resource goes with the texture.
    fn dispose(&self) {}
}

impl IPlatformRenderInterfaceImportedImage for VelloD3d12ImportedImage {
    fn acquire_shared_bitmap(&self, frame: FrameLease) -> Result<Arc<SharedBitmapImpl>, FrameError> {
        let m = frame.metadata();
        m.validate(MAX_FRAME_SIDE)?;
        if self.device.is_lost() || !frame.is_current() || m.device_generation != self.generation {
            return Err(FrameError::StaleGeneration);
        }
        let texture = self.texture.as_ref().map_err(|_| FrameError::MissingImporter)?;
        if !m.top_left_origin || m.width != texture.width() || m.height != texture.height() {
            return Err(FrameError::InvalidDimensions);
        }
        if m.device != Some(DeviceIdentity::Luid(self.luid.clone())) {
            return Err(FrameError::DeviceMismatch);
        }
        match m.color {
            FrameColor::Srgb => {}
            FrameColor::Rec709Full if m.alpha == FrameAlpha::Opaque => {}
            _ => return Err(FrameError::UnsupportedColor),
        }
        if m.format != FrameFormat::Bgra8
            || self.format != PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm
            || m.alpha == FrameAlpha::Straight
        {
            return Err(FrameError::UnsupportedFormat);
        }
        frame.counters().gpu_import();
        Ok(Arc::new(VelloSharedFrameBitmap::new(texture.clone(), frame)))
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

impl IExternalObjectsRenderInterfaceContextFeature for VelloD3d12ExternalObjectsFeature {
    fn shared_context_generation(&self) -> u64 {
        self.generation
    }

    fn supported_image_handle_types(&self) -> Vec<String> {
        vec![HandleTypes::D3D11_TEXTURE_NT_HANDLE.into()]
    }

    fn supported_semaphore_types(&self) -> Vec<String> {
        Vec::new()
    }

    /// A texture that cannot be opened gives an image whose frames are refused.
    ///
    /// # Panics
    /// Panics for a handle that is not an NT handle of a texture.
    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        if handle.handle_descriptor() != Some(HandleTypes::D3D11_TEXTURE_NT_HANDLE) {
            panic!("Importing this platform handle is not supported");
        }
        let (width, height) = (properties.width.max(0) as u32, properties.height.max(0) as u32);
        let texture = self.open(handle.handle(), width, height);
        if let Err(error) = &texture {
            report_import_failure(error);
        }
        Rc::new(VelloD3d12ImportedImage {
            device: self.device.clone(),
            generation: self.generation,
            luid: self.luid.clone(),
            texture,
            format: properties.format,
        })
    }

    /// # Panics
    /// Always: frames are imported by their handle.
    fn import_shared_image(
        &self,
        _image: Arc<dyn ICompositionImportableSharedGpuContextImage>,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        panic!("Specified method is not supported.");
    }

    /// # Panics
    /// Always: the producer hands a frame over once its copy has completed.
    fn import_semaphore(&self, _handle: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore> {
        panic!("Specified method is not supported.");
    }

    fn get_synchronization_capabilities(
        &self,
        image_handle_type: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        if image_handle_type == HandleTypes::D3D11_TEXTURE_NT_HANDLE {
            return CompositionGpuImportedImageSynchronizationCapabilities::AUTOMATIC;
        }
        CompositionGpuImportedImageSynchronizationCapabilities::default()
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        None
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        Some(self.luid.clone())
    }
}
