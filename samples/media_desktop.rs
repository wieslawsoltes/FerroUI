//! Shared desktop provider bindings for the framework samples. The renderer
//! determines admission; no sample opens an independent decode/capture pipeline.
use ferroui_base::rendering::composition::{frames::*, ICompositionGpuInterop};
use ferroui_media::{MediaError, MediaErrorKind};
use ferroui_media_camera::{CameraError, CameraErrorKind};
use std::rc::Rc;
fn unavailable(message: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::Unsupported, message)
}
fn camera_error(error: MediaError) -> CameraError {
    CameraError::new(CameraErrorKind::Unsupported, error.to_string())
}
pub fn video(
    interop: &dyn ICompositionGpuInterop,
    budget: FrameBudget,
) -> Result<
    (
        Rc<dyn ferroui_media_video::MediaPlayerProvider>,
        Rc<dyn ferroui_media_controls::TextureFrameImporter>,
    ),
    MediaError,
> {
    #[cfg(target_vendor = "apple")]
    {
        if interop.shared_context_id().is_some() {
            return Err(unavailable(
                "The catalog uses the Metal media profile on Apple platforms.",
            ));
        }
        let device = DeviceIdentity::Luid(
            interop
                .device_luid()
                .ok_or_else(|| unavailable("No admitted Metal device"))?,
        );
        Ok((
            Rc::new(ferroui_media_video_apple::AppleVideoProvider {
                device,
                device_generation: interop.shared_context_generation(),
                budget,
            }),
            Rc::new(ferroui_media_apple::AppleTextureImporter),
        ))
    }
    #[cfg(windows)]
    {
        Ok((
            Rc::new(ferroui_media_video_windows::WindowsVideoProvider {
                frames: ferroui_media_opengl::NativeGlFrameProducer::from_interop(interop, budget)?,
            }),
            Rc::new(ferroui_media_windows::NativeGlTextureImporter),
        ))
    }
    #[cfg(target_os = "linux")]
    {
        Ok((
            Rc::new(ferroui_media_video_linux::LinuxVideoProvider {
                frames: ferroui_media_opengl::NativeGlFrameProducer::from_interop(interop, budget)?,
            }),
            Rc::new(ferroui_media_linux::NativeGlTextureImporter),
        ))
    }
}
pub fn camera(
    interop: &dyn ICompositionGpuInterop,
    budget: FrameBudget,
) -> Result<
    (
        Rc<dyn ferroui_media_camera::CameraProvider>,
        Rc<dyn ferroui_media_controls::TextureFrameImporter>,
    ),
    CameraError,
> {
    #[cfg(target_vendor = "apple")]
    {
        let _ = budget;
        if interop.shared_context_id().is_some() {
            return Err(camera_error(unavailable(
                "The catalog uses the Metal camera profile on Apple platforms.",
            )));
        }
        let device = DeviceIdentity::Luid(
            interop
                .device_luid()
                .ok_or_else(|| camera_error(unavailable("No admitted Metal device")))?,
        );
        Ok((
            Rc::new(ferroui_media_camera_apple::AppleCameraProvider {
                device,
                device_generation: interop.shared_context_generation(),
            }),
            Rc::new(ferroui_media_apple::AppleTextureImporter),
        ))
    }
    #[cfg(windows)]
    {
        Ok((
            Rc::new(ferroui_media_camera_windows::WindowsCameraProvider::new(
                ferroui_media_opengl::NativeGlFrameProducer::from_interop(interop, budget)
                    .map_err(camera_error)?,
            )),
            Rc::new(ferroui_media_windows::NativeGlTextureImporter),
        ))
    }
    #[cfg(target_os = "linux")]
    {
        Ok((
            Rc::new(ferroui_media_camera_linux::LinuxCameraProvider::new(
                ferroui_media_opengl::NativeGlFrameProducer::from_interop(interop, budget)
                    .map_err(camera_error)?,
                ferroui_media_camera_linux::LinuxCameraProfile::Portal,
            )),
            Rc::new(ferroui_media_linux::NativeGlTextureImporter),
        ))
    }
}
