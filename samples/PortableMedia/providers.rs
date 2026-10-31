use ferroui_base::rendering::composition::{frames::*, ICompositionGpuInterop};
use ferroui_media::{MediaError, MediaErrorKind};
use ferroui_media_camera::CameraProvider;
use ferroui_media_controls::TextureFrameImporter;
use ferroui_media_video::MediaPlayerProvider;
use std::rc::Rc;
pub fn importer() -> Rc<dyn TextureFrameImporter> {
    #[cfg(target_os = "macos")]
    {
        Rc::new(ferroui_media_apple::AppleTextureImporter)
    }
    #[cfg(windows)]
    {
        Rc::new(ferroui_media_windows::NativeGlTextureImporter)
    }
    #[cfg(target_os = "linux")]
    {
        Rc::new(ferroui_media_linux::NativeGlTextureImporter)
    }
}
pub fn video(
    interop: &dyn ICompositionGpuInterop,
    budget: FrameBudget,
) -> Result<Rc<dyn MediaPlayerProvider>, MediaError> {
    #[cfg(target_os = "macos")]
    {
        Ok(Rc::new(ferroui_media_video_apple::AppleVideoProvider {
            device: DeviceIdentity::Luid(interop.device_luid().ok_or_else(|| {
                MediaError::new(MediaErrorKind::Unsupported, "No admitted Metal device")
            })?),
            device_generation: interop.shared_context_generation(),
            budget,
        }))
    }
    #[cfg(windows)]
    {
        Ok(Rc::new(ferroui_media_video_windows::WindowsVideoProvider {
            frames: ferroui_media_opengl::NativeGlFrameProducer::from_interop(interop, budget)?,
        }))
    }
    #[cfg(target_os = "linux")]
    {
        Ok(Rc::new(ferroui_media_video_linux::LinuxVideoProvider {
            frames: ferroui_media_opengl::NativeGlFrameProducer::from_interop(interop, budget)?,
        }))
    }
}
pub fn camera(
    interop: &dyn ICompositionGpuInterop,
    budget: FrameBudget,
) -> Result<Rc<dyn CameraProvider>, MediaError> {
    #[cfg(target_os = "macos")]
    {
        Ok(Rc::new(ferroui_media_camera_apple::AppleCameraProvider {
            device: DeviceIdentity::Luid(interop.device_luid().ok_or_else(|| {
                MediaError::new(MediaErrorKind::Unsupported, "No admitted Metal device")
            })?),
            device_generation: interop.shared_context_generation(),
        }))
    }
    #[cfg(windows)]
    {
        Ok(Rc::new(
            ferroui_media_camera_windows::WindowsCameraProvider::new(
                ferroui_media_opengl::NativeGlFrameProducer::from_interop(interop, budget)?,
            ),
        ))
    }
    #[cfg(target_os = "linux")]
    {
        let profile =
            match std::env::args().find_map(|arg| arg.strip_prefix("--v4l2=").map(str::to_owned)) {
                Some(path) => ferroui_media_camera_linux::LinuxCameraProfile::DirectV4l2 {
                    devices: vec![path.into()],
                },
                None => ferroui_media_camera_linux::LinuxCameraProfile::Portal,
            };
        Ok(Rc::new(
            ferroui_media_camera_linux::LinuxCameraProvider::new(
                ferroui_media_opengl::NativeGlFrameProducer::from_interop(interop, budget)?,
                profile,
            ),
        ))
    }
}
