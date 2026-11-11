//! Immutable D3D11 frame leases; conversion retires at an actual GPU event query.
use crate::{
    bitmap::Bitmap,
    ffi::{self, Owned},
};
use ferroui_base::{
    platform::*,
    rendering::composition::{frames::*, *},
    PixelSize, Vector,
};
use std::{
    any::Any,
    rc::Rc,
    sync::{Arc, Mutex},
};

pub const SHARED_D3D11_TEXTURE: &str = "SharedFrameD3D11Texture2D";

/// Returns one AddRef-owned ID3D11Device for the native media provider.
pub fn native_device() -> *mut std::ffi::c_void {
    unsafe { ffi::d2_native_device() }
}
pub fn device_luid() -> Vec<u8> {
    let mut result = vec![0; 8];
    unsafe {
        ffi::d2_device_luid(result.as_mut_ptr());
    }
    result
}
struct TextureInner {
    native: Mutex<Owned>,
    _owner: Box<dyn Any + Send + Sync>,
}
/// An immutable sampling view. The owner must retain producer storage through the
/// last lease and GPU read; an AddRef alone does not prevent a decoder reusing it.
#[derive(Clone)]
pub struct D3D11Texture(Arc<TextureInner>);
impl D3D11Texture {
    /// # Safety
    /// `texture` is a live ID3D11Texture2D on the native renderer's device. The
    /// selected slice is fully produced, immutable, and retained by `owner`.
    pub unsafe fn from_texture<T: Any + Send + Sync>(
        texture: *mut std::ffi::c_void,
        slice: u32,
        owner: T,
    ) -> Option<Self> {
        let native = ffi::d2_texture(texture, slice);
        (!native.is_null()).then(|| {
            Self(Arc::new(TextureInner {
                native: Mutex::new(Owned::new(native)),
                _owner: Box::new(owner),
            }))
        })
    }
}
struct TextureHandle(D3D11Texture);
impl IPlatformHandle for TextureHandle {
    fn handle(&self) -> isize {
        self.0 .0.native.lock().unwrap().0 as isize
    }
    fn handle_descriptor(&self) -> Option<&str> {
        Some(SHARED_D3D11_TEXTURE)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
pub struct ExternalObjects {
    pub(crate) generation: u64,
}
impl IExternalObjectsRenderInterfaceContextFeature for ExternalObjects {
    fn shared_context_generation(&self) -> u64 {
        self.generation
    }
    fn supported_image_handle_types(&self) -> Vec<String> {
        vec![
            SHARED_D3D11_TEXTURE.into(),
            KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_NT_HANDLE.into(),
            KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_GLOBAL_SHARED_HANDLE
                .into(),
        ]
    }
    fn supported_semaphore_types(&self) -> Vec<String> {
        if unsafe { ffi::d2_fences_supported() } != 0 {
            vec![
                KnownPlatformGraphicsExternalSemaphoreHandleTypes::DIRECT3D12_FENCE_NT_HANDLE
                    .into(),
            ]
        } else {
            vec![]
        }
    }
    fn import_image(
        &self,
        handle: Rc<dyn IPlatformHandle>,
        properties: PlatformGraphicsExternalImageProperties,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        let frame = handle
            .as_any()
            .downcast_ref::<LeasedPlatformHandle>()
            .map(|h| h.frame().clone());
        let (texture, immutable) = if let Some(frame) = &frame {
            (
                frame
                    .resource::<D3D11Texture>()
                    .expect("A native D3D11 handle requires its immutable texture lease")
                    .clone(),
                true,
            )
        } else if let Some(native) = handle.as_any().downcast_ref::<TextureHandle>() {
            (native.0.clone(), true)
        } else {
            let nt=match handle.handle_descriptor() {
                Some(KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_NT_HANDLE)=>true,
                Some(KnownPlatformGraphicsExternalImageHandleTypes::D3D11_TEXTURE_GLOBAL_SHARED_HANDLE)=>false,
                _=>panic!("Unsupported Direct2D external texture handle"),
            };
            let native = unsafe { Owned::new(ffi::d2_open_texture(handle.handle(), nt as i32)) };
            (
                D3D11Texture(Arc::new(TextureInner {
                    native: Mutex::new(native),
                    _owner: Box::new(()),
                })),
                false,
            )
        };
        Rc::new(ImportedImage {
            texture,
            properties,
            immutable,
            generation: self.generation,
            _frame: frame,
        })
    }
    fn import_shared_image(
        &self,
        _: Arc<dyn ICompositionImportableSharedGpuContextImage>,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedImage> {
        panic!("Use Direct2D's leased D3D11 texture handle")
    }
    fn import_semaphore(
        &self,
        handle: Rc<dyn IPlatformHandle>,
    ) -> Rc<dyn IPlatformRenderInterfaceImportedSemaphore> {
        assert_eq!(
            handle.handle_descriptor(),
            Some(KnownPlatformGraphicsExternalSemaphoreHandleTypes::DIRECT3D12_FENCE_NT_HANDLE)
        );
        Rc::new(ImportedFence(unsafe {
            Owned::new(ffi::d2_open_fence(handle.handle()))
        }))
    }
    fn get_synchronization_capabilities(
        &self,
        kind: &str,
    ) -> CompositionGpuImportedImageSynchronizationCapabilities {
        if kind == SHARED_D3D11_TEXTURE {
            CompositionGpuImportedImageSynchronizationCapabilities::AUTOMATIC
        } else if self
            .supported_image_handle_types()
            .iter()
            .any(|v| v == kind)
        {
            let mut caps = CompositionGpuImportedImageSynchronizationCapabilities::KEYED_MUTEX;
            if unsafe { ffi::d2_fences_supported() } != 0 {
                caps |= CompositionGpuImportedImageSynchronizationCapabilities::TIMELINE_SEMAPHORES;
            }
            caps
        } else {
            CompositionGpuImportedImageSynchronizationCapabilities::empty()
        }
    }
    fn device_uuid(&self) -> Option<Vec<u8>> {
        None
    }
    fn device_luid(&self) -> Option<Vec<u8>> {
        Some(device_luid())
    }
}
struct ImportedImage {
    texture: D3D11Texture,
    properties: PlatformGraphicsExternalImageProperties,
    immutable: bool,
    generation: u64,
    _frame: Option<FrameLease>,
}
struct ImportedFence(Owned);
impl IPlatformRenderInterfaceImportedObject for ImportedFence {
    fn dispose(&self) {}
}
impl IPlatformRenderInterfaceImportedSemaphore for ImportedFence {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
impl ImportedImage {
    fn snapshot(
        &self,
        key: Option<(u64, u64)>,
        wait: Option<(&Rc<dyn IPlatformRenderInterfaceImportedSemaphore>, u64)>,
        signal: Option<(&Rc<dyn IPlatformRenderInterfaceImportedSemaphore>, u64)>,
    ) -> Arc<SharedBitmapImpl> {
        let p = &self.properties;
        let texture = self.texture.0.native.lock().unwrap();
        assert!(
            p.width > 0
                && p.height > 0
                && unsafe {
                    ffi::d2_texture_valid(
                        texture.0,
                        p.width as u32,
                        p.height as u32,
                        (p.format == PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm) as i32,
                    )
                } != 0,
            "External texture dimensions/format do not match"
        );
        let output = Bitmap::new(PixelSize::new(p.width, p.height), Vector::new(96., 96.));
        let fence = |v: Option<(&Rc<dyn IPlatformRenderInterfaceImportedSemaphore>, u64)>| {
            v.map_or((std::ptr::null_mut(), 0), |(f, value)| {
                (
                    f.as_any()
                        .downcast_ref::<ImportedFence>()
                        .expect("Native D3D11 fence")
                        .0
                         .0,
                    value,
                )
            })
        };
        let (wait, wv) = fence(wait);
        let (signal, sv) = fence(signal);
        let (acquire, release) = key.unwrap_or_default();
        let result = output.context(|dest| unsafe {
            ffi::d2_copy_external(
                dest,
                texture.0,
                key.is_some() as i32,
                acquire,
                release,
                wait,
                wv,
                signal,
                sv,
                (!p.top_left_origin) as i32,
            )
        });
        assert_ne!(
            result, 0,
            "External texture is not ready, invalid, or its synchronization failed"
        );
        Arc::new(output)
    }
}
impl IPlatformRenderInterfaceImportedObject for ImportedImage {
    fn dispose(&self) {}
}
impl IPlatformRenderInterfaceImportedImage for ImportedImage {
    fn acquire_shared_bitmap(
        &self,
        frame: FrameLease,
    ) -> Result<Arc<SharedBitmapImpl>, FrameError> {
        let m = frame.metadata();
        m.validate(8192)?;
        if !frame.is_current()
            || m.device_generation != self.generation
            || self.generation != crate::device_generation()
        {
            return Err(FrameError::StaleGeneration);
        }
        if m.device != Some(DeviceIdentity::Luid(device_luid())) {
            return Err(FrameError::DeviceMismatch);
        }
        if m.width != self.properties.width as u32
            || m.height != self.properties.height as u32
            || m.top_left_origin != self.properties.top_left_origin
        {
            return Err(FrameError::InvalidDimensions);
        }
        if m.alpha == FrameAlpha::Straight {
            return Err(FrameError::UnsupportedFormat);
        }
        if m.color != FrameColor::Srgb
            && !(m.color == FrameColor::Rec709Full && m.alpha == FrameAlpha::Opaque)
        {
            return Err(FrameError::UnsupportedColor);
        }
        let rgba = m.format == FrameFormat::Rgba8;
        if !matches!(m.format, FrameFormat::Rgba8 | FrameFormat::Bgra8)
            || unsafe {
                ffi::d2_texture_valid(
                    self.texture.0.native.lock().unwrap().0,
                    m.width,
                    m.height,
                    rgba as i32,
                )
            } == 0
        {
            return Err(FrameError::UnsupportedFormat);
        }
        frame.counters().gpu_import();
        Ok(Arc::new(SharedFrameBitmap {
            size: PixelSize::new(m.width as i32, m.height as i32),
            state: Mutex::new(FrameState {
                texture: Some(self.texture.clone()),
                frame: Some(frame),
                converted: None,
            }),
        }))
    }
    fn snapshot_with_keyed_mutex(&self, acquire: u32, release: u32) -> Arc<SharedBitmapImpl> {
        self.snapshot(Some((acquire as u64, release as u64)), None, None)
    }
    fn snapshot_with_semaphores(
        &self,
        _: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        _: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
    ) -> Arc<SharedBitmapImpl> {
        panic!("Binary semaphore import is unavailable")
    }
    fn snapshot_with_timeline_semaphores(
        &self,
        wait: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        wait_value: u64,
        signal: &Rc<dyn IPlatformRenderInterfaceImportedSemaphore>,
        signal_value: u64,
    ) -> Arc<SharedBitmapImpl> {
        self.snapshot(None, Some((wait, wait_value)), Some((signal, signal_value)))
    }
    fn snapshot_with_automatic_sync(&self) -> Arc<SharedBitmapImpl> {
        assert!(
            self.immutable,
            "Shared textures require keyed mutex or timeline fences"
        );
        self.snapshot(None, None, None)
    }
}
pub(crate) struct SharedFrameBitmap {
    size: PixelSize,
    state: Mutex<FrameState>,
}
struct FrameState {
    texture: Option<D3D11Texture>,
    frame: Option<FrameLease>,
    converted: Option<Bitmap>,
}
impl SharedFrameBitmap {
    pub(crate) fn bitmap(&self) -> Option<Bitmap> {
        let mut state = self.state.lock().unwrap();
        if let Some(converted) = &state.converted {
            return (!converted.is_corrupted()).then(|| converted.clone());
        }
        let frame = state.frame.as_ref()?.clone();
        if !frame.is_current() {
            return None;
        }
        let m = frame.metadata();
        let completion = GpuCompletion::new();
        completion.retain(frame.clone()).ok()?;
        let output = Bitmap::new(self.size, Vector::new(96., 96.));
        let callback = Box::into_raw(Box::new(completion.clone())).cast();
        let submitted = output.context(|p| unsafe {
            ffi::d2_convert_frame(
                p,
                state.texture.as_ref().unwrap().0.native.lock().unwrap().0,
                (m.color == FrameColor::Rec709Full) as i32,
                (!m.top_left_origin) as i32,
                (m.alpha == FrameAlpha::Opaque) as i32,
                callback,
                completed,
            )
        });
        if submitted == 0 {
            unsafe {
                drop(Box::from_raw(callback.cast::<GpuCompletion>()));
            }
            // The native bridge guarantees that no source read was submitted on failure.
            completion.complete();
            return None;
        }
        if submitted < 0 {
            return None;
        }
        frame
            .counters()
            .gpu_copy(GpuTransferReason::NativeImageTransfer);
        if m.color == FrameColor::Rec709Full {
            frame
                .counters()
                .gpu_conversion(GpuTransferReason::ColorSpaceConversion);
        }
        state.frame = None;
        state.texture = None;
        state.converted = Some(output.clone());
        Some(output)
    }
}
extern "C" fn completed(context: ffi::Ptr, success: i32) {
    let completion = unsafe { Box::from_raw(context.cast::<GpuCompletion>()) };
    if success != 0 {
        completion.complete();
    } else {
        completion.mark_indeterminate();
    }
}
impl IBitmapImpl for SharedFrameBitmap {
    fn dpi(&self) -> Vector {
        Vector::new(96., 96.)
    }
    fn pixel_size(&self) -> PixelSize {
        self.size
    }
    fn version(&self) -> i32 {
        1
    }
    fn save(
        &self,
        _: &mut dyn std::io::Write,
        _: &ferroui_base::media::imaging::BitmapEncoderOptions,
    ) -> std::io::Result<()> {
        Err(std::io::Error::other(
            "GPU frame export requires explicit counted capture",
        ))
    }
    fn dispose(&self) {
        let mut state = self.state.lock().unwrap();
        state.texture = None;
        state.frame = None;
        state.converted = None;
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[cfg(feature = "media-controls")]
pub struct D3D11TextureImporter;
#[cfg(feature = "media-controls")]
impl ferroui_media_controls::TextureFrameImporter for D3D11TextureImporter {
    fn import(
        &self,
        interop: &dyn ICompositionGpuInterop,
        frame: &FrameLease,
    ) -> Result<ferroui_media_controls::ImportedTextureFrame, ferroui_media::MediaError> {
        use ferroui_media::{MediaError, MediaErrorKind};
        let error = |s| MediaError::new(MediaErrorKind::Unsupported, s);
        let m = frame.metadata();
        if m.device != interop.device_luid().map(DeviceIdentity::Luid)
            || m.device_generation != interop.shared_context_generation()
        {
            return Err(error("D3D11 frame and renderer devices differ"));
        }
        let texture = frame
            .resource::<D3D11Texture>()
            .ok_or_else(|| error("No leased D3D11 storage"))?;
        let handle = LeasedPlatformHandle::new(
            texture.0.native.lock().unwrap().0 as isize,
            Some(SHARED_D3D11_TEXTURE),
            frame.clone(),
        );
        let image = interop.import_image(
            Rc::new(handle),
            PlatformGraphicsExternalImageProperties {
                width: m.width as i32,
                height: m.height as i32,
                format: if m.format == FrameFormat::Rgba8 {
                    PlatformGraphicsExternalImageFormat::R8G8B8A8UNorm
                } else {
                    PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm
                },
                top_left_origin: m.top_left_origin,
                memory_size: m.retained_bytes,
                ..Default::default()
            },
        );
        let producer_ready = GpuCompletion::new();
        producer_ready.complete();
        Ok(ferroui_media_controls::ImportedTextureFrame {
            image,
            producer_ready,
        })
    }
}
