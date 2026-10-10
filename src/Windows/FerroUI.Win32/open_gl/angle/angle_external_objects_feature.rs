//! The external objects of a context of ANGLE on Direct3D 11: textures of
//! Direct3D 11 that are created for, or imported from, another graphics
//! API through a shared handle.

use super::angle_external_d3d11_texture2_d::{AngleExternalMemoryD3D11ExportedTexture2D, AngleExternalMemoryD3D11Texture2D};
use super::AngleWin32EglDisplay;
use crate::direct_x::{
    ID3D11Device, ID3D11Device1, ID3D11Texture2D, IDXGIDevice, D3D11_BIND_FLAG, D3D11_RESOURCE_MISC_FLAG,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE, DXGI_FORMAT, DXGI_SAMPLE_DESC,
};
use ferroui_base::platform::{
    IPlatformGraphicsContext, IPlatformHandle, KnownPlatformGraphicsExternalImageHandleTypes, PlatformGraphicsExternalImageFormat,
    PlatformGraphicsExternalImageProperties,
};
use ferroui_base::rendering::composition::CompositionGpuImportedImageSynchronizationCapabilities;
use ferroui_base::PixelSize;
use ferroui_microcom::{ComPtr, HResult, Interface};
use ferroui_opengl::egl::EglContext;
use ferroui_opengl::{
    IGlContextExternalObjectsFeature, IGlExportableExternalImageTexture, IGlExternalImageTexture, IGlExternalSemaphore,
    OpenGlException,
};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The handle types of images the feature imports and exports.
pub(crate) const SUPPORTED_IMAGE_TYPES: [&str; 2] = [
    KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_GLOBAL_SHARED_HANDLE,
    KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_NT_HANDLE,
];

/// The capabilities for a type of image handle: a keyed mutex for the two
/// types of Direct3D 11, none for any other.
pub(crate) fn synchronization_capabilities(image_handle_type: &str) -> CompositionGpuImportedImageSynchronizationCapabilities {
    if SUPPORTED_IMAGE_TYPES.contains(&image_handle_type) {
        CompositionGpuImportedImageSynchronizationCapabilities::KEYED_MUTEX
    } else {
        CompositionGpuImportedImageSynchronizationCapabilities::default()
    }
}

struct Devices {
    device: ComPtr<ID3D11Device>,
    device1: ComPtr<ID3D11Device1>,
}

/// The feature of one context. The context holds the feature among its
/// features, so the feature refers to its context weakly.
pub(crate) struct AngleExternalObjectsFeature {
    context: Weak<EglContext>,
    /// `None` once disposed.
    devices: RefCell<Option<Devices>>,
    device_luid: Vec<u8>,
}

fn failure(what: &str) -> impl Fn(HResult) -> OpenGlException + '_ {
    move |error| OpenGlException::new(format!("{what}: {error}"))
}

impl AngleExternalObjectsFeature {
    pub fn new(context: &Rc<EglContext>, angle: &AngleWin32EglDisplay) -> Result<AngleExternalObjectsFeature, OpenGlException> {
        let device = angle.get_direct3d_device()?;
        // SAFETY: the display answers with a pointer to its device of
        // Direct3D 11, which it owns; the feature takes a reference of its
        // own.
        let device = unsafe { ComPtr::<ID3D11Device>::from_raw_add_ref(device as *mut ID3D11Device) }
            .ok_or_else(|| OpenGlException::new("The display has no Direct3D 11 device"))?;
        let device1 = device.cast::<ID3D11Device1>().map_err(failure("ID3D11Device1"))?;
        let dxgi_device = device.cast::<IDXGIDevice>().map_err(failure("IDXGIDevice"))?;
        let adapter = dxgi_device
            .get_adapter()
            .and_then(|adapter| adapter.ok_or(HResult::POINTER))
            .map_err(failure("IDXGIDevice::GetAdapter"))?;
        let device_luid = adapter.get_desc().map_err(failure("IDXGIAdapter::GetDesc"))?.adapter_luid.to_le_bytes().to_vec();

        Ok(AngleExternalObjectsFeature {
            context: Rc::downgrade(context),
            devices: RefCell::new(Some(Devices { device, device1 })),
            device_luid,
        })
    }

    fn context(&self) -> Result<Rc<EglContext>, OpenGlException> {
        self.context.upgrade().ok_or_else(|| OpenGlException::new("The context of the feature was disposed"))
    }

    /// Releases the device.
    pub fn dispose(&self) {
        *self.devices.borrow_mut() = None;
    }
}

impl IGlContextExternalObjectsFeature for AngleExternalObjectsFeature {
    fn supported_importable_external_image_types(&self) -> Vec<String> {
        SUPPORTED_IMAGE_TYPES.iter().map(|&name| name.to_owned()).collect()
    }

    fn supported_exportable_external_image_types(&self) -> Vec<String> {
        self.supported_importable_external_image_types()
    }

    fn supported_importable_external_semaphore_types(&self) -> Vec<String> {
        Vec::new()
    }

    fn supported_exportable_external_semaphore_types(&self) -> Vec<String> {
        Vec::new()
    }

    fn get_supported_formats_for_external_memory_type(&self, _type: &str) -> Vec<PlatformGraphicsExternalImageFormat> {
        vec![PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm]
    }

    fn create_image(
        &self,
        _type: &str,
        size: PixelSize,
        format: PlatformGraphicsExternalImageFormat,
    ) -> Result<Rc<dyn IGlExportableExternalImageTexture>, OpenGlException> {
        if format != PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm {
            return Err(OpenGlException::new("Unsupported external memory format"));
        }

        let context = self.context()?;
        let devices = self.devices.borrow();
        let devices = devices.as_ref().ok_or_else(|| OpenGlException::new("The feature was disposed"))?;
        let current = IPlatformGraphicsContext::ensure_current(&*context);

        let mut desc = D3D11_TEXTURE2D_DESC {
            format: DXGI_FORMAT::DXGI_FORMAT_R8G8B8A8_UNORM,
            width: size.width as u32,
            height: size.height as u32,
            array_size: 1,
            mip_levels: 1,
            sample_desc: DXGI_SAMPLE_DESC { count: 1, quality: 0 },
            usage: D3D11_USAGE::D3D11_USAGE_DEFAULT,
            cpu_access_flags: 0,
            misc_flags: D3D11_RESOURCE_MISC_FLAG::D3D11_RESOURCE_MISC_SHARED_KEYEDMUTEX,
            bind_flags: D3D11_BIND_FLAG::D3D11_BIND_RENDER_TARGET | D3D11_BIND_FLAG::D3D11_BIND_SHADER_RESOURCE,
        };
        // SAFETY: the description is a value of this frame that lives
        // through the call; no initial data is given.
        let result = unsafe { devices.device.create_texture2d(&mut desc, 0) }
            .and_then(|texture| texture.ok_or(HResult::POINTER))
            .map_err(failure("ID3D11Device::CreateTexture2D"))
            .and_then(|texture| AngleExternalMemoryD3D11ExportedTexture2D::new(&context, &texture, &desc, format));
        current.dispose();

        let texture: Rc<dyn IGlExportableExternalImageTexture> = Rc::new(result?);
        Ok(texture)
    }

    fn create_semaphore(&self, _type: &str) -> Result<Rc<dyn IGlExportableExternalImageTexture>, OpenGlException> {
        Err(OpenGlException::new("Specified method is not supported."))
    }

    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Result<Rc<dyn IGlExternalImageTexture>, OpenGlException> {
        let descriptor = handle.handle_descriptor();
        let descriptor = descriptor.as_deref().unwrap_or_default();
        if !SUPPORTED_IMAGE_TYPES.contains(&descriptor) {
            return Err(OpenGlException::new("Unsupported external memory type"));
        }

        let context = self.context()?;
        let devices = self.devices.borrow();
        let devices = devices.as_ref().ok_or_else(|| OpenGlException::new("The feature was disposed"))?;
        let current = IPlatformGraphicsContext::ensure_current(&*context);

        let mut guid = ID3D11Texture2D::IID;
        // SAFETY (both calls): the identifier is a value of this frame
        // that lives through the call; the handle is the one the caller
        // names as a shared handle of a texture of Direct3D 11.
        let opened = if descriptor == KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_GLOBAL_SHARED_HANDLE {
            unsafe { devices.device.open_shared_resource(handle.handle(), &mut guid) }
        } else {
            unsafe { devices.device1.open_shared_resource1(handle.handle(), &mut guid) }
        };
        let result = opened
            .and_then(|opened| opened.ok_or(HResult::POINTER))
            .and_then(|opened| opened.cast::<ID3D11Texture2D>())
            .map_err(failure("ID3D11Device::OpenSharedResource"))
            .and_then(|texture| AngleExternalMemoryD3D11Texture2D::new(&context, &texture, properties));
        current.dispose();

        let texture: Rc<dyn IGlExternalImageTexture> = Rc::new(result?);
        Ok(texture)
    }

    fn import_semaphore(&self, _handle: Rc<dyn IPlatformHandle>) -> Result<Rc<dyn IGlExternalSemaphore>, OpenGlException> {
        Err(OpenGlException::new("Specified method is not supported."))
    }

    fn get_synchronization_capabilities(&self, image_handle_type: &str) -> CompositionGpuImportedImageSynchronizationCapabilities {
        synchronization_capabilities(image_handle_type)
    }

    fn device_luid(&self) -> Option<Vec<u8>> {
        Some(self.device_luid.clone())
    }

    fn device_uuid(&self) -> Option<Vec<u8>> {
        None
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the feature.
    use super::*;

    #[test]
    fn the_images_of_direct3d_11_are_synchronized_with_a_keyed_mutex() {
        assert_eq!(
            CompositionGpuImportedImageSynchronizationCapabilities::KEYED_MUTEX,
            synchronization_capabilities("D3D11TextureGlobalSharedHandle")
        );
        assert_eq!(
            CompositionGpuImportedImageSynchronizationCapabilities::KEYED_MUTEX,
            synchronization_capabilities("D3D11TextureNtHandle")
        );
        assert_eq!(
            CompositionGpuImportedImageSynchronizationCapabilities::default(),
            synchronization_capabilities("VulkanOpaquePosixFileDescriptor")
        );
    }
}
