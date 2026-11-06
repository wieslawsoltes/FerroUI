//! CoreVideo ownership and IOSurface presentation shared by video and camera.
#![cfg(target_vendor = "apple")]
#![deny(unsafe_code)]
#[allow(unsafe_code)]
mod native;
#[cfg(all(target_os = "macos", feature = "opengl"))]
mod cgl;
#[cfg(all(target_os = "macos", feature = "opengl"))]
pub use cgl::CglFrameView;
use ferroui_base::{
    platform::{
        IPlatformHandle, KnownPlatformGraphicsExternalImageHandleTypes, LeasedPlatformHandle,
    },
    reactive::Subscription,
    threading::{DispatcherPriority, DispatcherTimer},
};
use ferroui_media::{frames::*, *};
pub use native::PixelBuffer;
use std::{rc::Rc, task::Poll, time::Duration};
pub fn poll_native<T: 'static, E: 'static>(
    mut read: impl FnMut() -> Poll<Result<T, E>> + 'static,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>>>> {
    let mut timer: Option<Subscription> = None;
    Box::pin(std::future::poll_fn(move |cx| {
        timer.take();
        match read() {
            Poll::Ready(result) => Poll::Ready(result),
            Poll::Pending => {
                let waker = cx.waker().clone();
                timer = Some(DispatcherTimer::run_once(
                    move || waker.wake_by_ref(),
                    Duration::from_millis(16),
                    DispatcherPriority::NORMAL,
                ));
                Poll::Pending
            }
        }
    }))
}
/// Returns an owning import handle; it keeps the decoder allocation leased even
/// if the source is replaced while the compositor import job is still queued.
pub fn frame_surface(frame: &FrameLease) -> Result<Rc<dyn IPlatformHandle>, MediaError> {
    let surface = frame
        .resource::<PixelBuffer>()
        .and_then(|buffer| buffer.surface())
        .ok_or_else(|| {
            MediaError::new(MediaErrorKind::Unsupported, "Not a CoreVideo-backed frame")
        })?;
    Ok(Rc::new(LeasedPlatformHandle::new(
        surface as isize,
        Some(KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF),
        frame.clone(),
    )))
}

#[cfg(feature = "controls")]
pub struct AppleTextureImporter;
#[cfg(feature = "controls")]
impl ferroui_media_controls::TextureFrameImporter for AppleTextureImporter {
    fn import(
        &self,
        interop: &dyn ferroui_base::rendering::composition::ICompositionGpuInterop,
        frame: &FrameLease,
    ) -> Result<ferroui_media_controls::ImportedTextureFrame, MediaError> {
        use ferroui_base::platform::{
            PlatformGraphicsExternalImageFormat, PlatformGraphicsExternalImageProperties,
        };
        let m = frame.metadata();
        #[cfg(all(target_os = "macos", feature = "opengl"))]
        if frame.resource::<ferroui_opengl::SharedFrameTexture>().is_some() {
            return ferroui_media_controls::TextureFrameImporter::import(
                &ferroui_media_opengl::NativeGlTextureImporter, interop, frame);
        }
        if !interop.supported_image_handle_types().iter().any(|t| t == KnownPlatformGraphicsExternalImageHandleTypes::IO_SURFACE_REF) {
            return Err(MediaError::new(MediaErrorKind::Unsupported, "Renderer cannot directly import CoreVideo IOSurfaces"));
        }
        if m.device_generation != interop.shared_context_generation() {
            return Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "Frame belongs to a retired renderer generation",
            ));
        }
        if Some(DeviceIdentity::Luid(interop.device_luid().ok_or_else(
            || {
                MediaError::new(
                    MediaErrorKind::Unsupported,
                    "Renderer device identity is unavailable",
                )
            },
        )?)) != m.device
        {
            return Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "Frame and renderer devices differ",
            ));
        }
        let image = interop.import_image(
            frame_surface(frame)?,
            PlatformGraphicsExternalImageProperties {
                width: m.width as i32,
                height: m.height as i32,
                format: PlatformGraphicsExternalImageFormat::B8G8R8A8UNorm,
                top_left_origin: true,
                memory_size: m.retained_bytes,
                ..Default::default()
            },
        );
        // CoreVideo has transferred a ready immutable image; this is the native
        // acquisition boundary, not evidence about opaque decoder internals.
        let producer_ready = GpuCompletion::new();
        producer_ready.complete();
        Ok(ferroui_media_controls::ImportedTextureFrame {
            image,
            producer_ready,
        })
    }
}
