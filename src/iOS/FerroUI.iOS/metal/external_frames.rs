//! Lease-backed IOSurface texture views on the active iOS Metal device.
//! No snapshots, CPU uploads or additional command queues are involved.
use super::MetalDevice;
use ferroui_base::{
    platform::{
        IPlatformHandle, KnownPlatformGraphicsExternalImageHandleTypes, LeasedPlatformHandle,
        PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties,
    },
    rendering::composition::CompositionGpuImportedImageSynchronizationCapabilities,
};
use ferroui_metal::{IMetalExternalObjectsFeature, IMetalExternalTexture, IMetalSharedEvent};
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_io_surface::IOSurfaceRef;
use objc2_metal::{
    MTLDevice, MTLPixelFormat, MTLStorageMode, MTLTexture, MTLTextureDescriptor, MTLTextureUsage,
};
use std::{cell::RefCell, ffi::c_void, rc::Rc};

impl IMetalExternalObjectsFeature for MetalDevice {
    fn supported_image_handle_types(&self) -> Vec<String> {
        vec![KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF.into()]
    }
    fn supported_semaphore_types(&self) -> Vec<String> {
        Vec::new()
    }
    fn device_luid(&self) -> Option<Vec<u8>> {
        Some(self.mtl_device().registryID().to_be_bytes().to_vec())
    }
    fn get_synchronization_capabilities(
        &self,
        _: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        CompositionGpuImportedImageSynchronizationCapabilities::empty()
    }
    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IMetalExternalTexture> {
        assert_eq!(
            handle.handle_descriptor(),
            Some(KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF)
        );
        assert!(
            handle.as_any().is::<LeasedPlatformHandle>(),
            "iOS surface import requires an owning frame lease"
        );
        assert_ne!(handle.handle(), 0, "Null IOSurface");
        // SAFETY: an IOSurface import handle backed by a live producer lease;
        // composition keeps that lease through this job and actual GPU reads.
        let surface = unsafe { &*(handle.handle() as *const IOSurfaceRef) };
        assert_eq!(
            (surface.width(), surface.height()),
            (properties.width as usize, properties.height as usize)
        );
        assert_eq!(
            surface.plane_count(),
            0,
            "Only non-planar RGBA/BGRA storage is admitted"
        );
        let format = match properties.format {
            PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm => {
                assert_eq!(surface.pixel_format(), u32::from_be_bytes(*b"BGRA"));
                MTLPixelFormat::BGRA8Unorm
            }
            PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm => {
                assert_eq!(surface.pixel_format(), u32::from_be_bytes(*b"RGBA"));
                MTLPixelFormat::RGBA8Unorm
            }
        };
        // SAFETY: validated two-dimensional non-planar storage, one mip level.
        let descriptor = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                format,
                surface.width(),
                surface.height(),
                false,
            )
        };
        descriptor.setStorageMode(MTLStorageMode::Shared);
        descriptor.setUsage(MTLTextureUsage::ShaderRead);
        let texture = self
            .mtl_device()
            .newTextureWithDescriptor_iosurface_plane(&descriptor, surface, 0)
            .expect("The active Metal device cannot view this IOSurface");
        Rc::new(Texture {
            width: properties.width,
            height: properties.height,
            texture: RefCell::new(Some(texture)),
            owner: RefCell::new(Some(handle)),
        })
    }
    fn import_shared_event(&self, _: Rc<dyn IPlatformHandle>) -> Rc<dyn IMetalSharedEvent> {
        panic!("Shared events are not advertised by this iOS profile")
    }
    fn submit_wait(&self, _: &dyn IMetalSharedEvent, _: u64) {
        panic!("Shared events are not advertised by this iOS profile")
    }
    fn submit_signal(&self, _: &dyn IMetalSharedEvent, _: u64) {
        panic!("Shared events are not advertised by this iOS profile")
    }
}
struct Texture {
    width: i32,
    height: i32,
    texture: RefCell<Option<Retained<ProtocolObject<dyn MTLTexture>>>>,
    owner: RefCell<Option<Rc<dyn IPlatformHandle>>>,
}
impl IMetalExternalTexture for Texture {
    fn width(&self) -> i32 {
        self.width
    }
    fn height(&self) -> i32 {
        self.height
    }
    fn samples(&self) -> i32 {
        1
    }
    fn handle(&self) -> *mut c_void {
        self.texture
            .borrow()
            .as_ref()
            .map(|v| Retained::as_ptr(v) as *mut c_void)
            .unwrap_or(std::ptr::null_mut())
    }
    fn dispose(&self) {
        self.texture.borrow_mut().take();
        self.owner.borrow_mut().take();
    }
}
