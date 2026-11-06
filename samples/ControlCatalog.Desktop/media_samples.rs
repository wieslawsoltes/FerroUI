//! Desktop catalog bindings to admitted native media providers.
use control_catalog::media_samples::MediaSampleProviders;
use ferroui_base::rendering::composition::{ICompositionGpuInterop, frames::*};
use ferroui_media::{MediaError, MediaErrorKind};
use ferroui_media_camera::{CameraError, CameraErrorKind};
use std::rc::Rc;

pub fn install() {
    control_catalog::media_samples::install(MediaSampleProviders {
        video: Rc::new(video),
        camera: Rc::new(camera),
        #[cfg(target_vendor = "apple")]
        qr: Some(Rc::new(ferroui_media_qr_apple::recognize)),
        #[cfg(not(target_vendor = "apple"))]
        qr: None,
    });
}
fn unavailable(message: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::Unsupported, message)
}
fn camera_error(error: MediaError) -> CameraError {
    CameraError::new(CameraErrorKind::Unsupported, error.to_string())
}
fn video(
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
fn camera(
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

pub static SMOKE_FAILED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn smoke() {
    if std::env::var("FERROUI_SMOKE_MEDIA").as_deref() != Ok("1") {
        return;
    }
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use std::{
        cell::Cell,
        time::{Duration, Instant},
    };
    let stage = Cell::new(0);
    let began = Instant::now();
    let old = std::cell::RefCell::new(None::<FrameBudget>);
    DispatcherTimer::run(
        move || {
            use control_catalog::media_samples::{smoke_action, smoke_snapshot};
            let fail = |reason: &str| {
                eprintln!("CATALOG_MEDIA_SMOKE_FAILED {reason}");
                SMOKE_FAILED.store(true, std::sync::atomic::Ordering::Relaxed);
                if let Some(window) = super::main_window() {
                    window.close();
                }
            };
            if began.elapsed() > Duration::from_secs(60) {
                fail(&format!(
                    "timeout stage={} status={:?}",
                    stage.get(),
                    smoke_snapshot().map(|s| s.status)
                ));
                return false;
            }
            match stage.get() {
                0 => {
                    if control_catalog::select_page("Video") {
                        stage.set(1);
                    }
                }
                1 => {
                    if smoke_action("load") {
                        stage.set(2);
                    }
                }
                2 => {
                    if let Some(s) =
                        smoke_snapshot().filter(|s| s.title == "Video" && s.paused && s.frame)
                    {
                        old.replace(Some(s.budget));
                        smoke_action("play");
                        stage.set(3);
                    }
                }
                3 => {
                    if let Some(s) = smoke_snapshot().filter(|s| {
                        s.playing && s.frame && s.budget.counters().snapshot().gpu_imports > 0
                    }) {
                        if s.budget.counters().snapshot().preview != PixelTransferCount::default() {
                            fail("CPU preview transfer");
                            return false;
                        }
                        println!("CATALOG_VIDEO_PASS autoplay=false shared_texture=true");
                        control_catalog::select_page("Camera");
                        stage.set(4);
                    }
                }
                4 => {
                    if old.borrow().as_ref().is_some_and(|b| {
                        b.usage().frames == 0 && b.gpu_reads() == 0 && b.quarantined_reads() == 0
                    }) && smoke_snapshot().is_some_and(|s| s.title == "Camera" && s.attached)
                    {
                        println!("CATALOG_VIDEO_NAVIGATION_RETIREMENT_PASS");
                        smoke_action("discover");
                        stage.set(5);
                    }
                }
                5 => {
                    if let Some(s) =
                        smoke_snapshot().filter(|s| s.status.contains("camera(s); permission"))
                    {
                        if s.camera_active {
                            fail("discovery acquired a camera");
                            return false;
                        }
                        println!("CATALOG_CAMERA_DISCOVERY_PASS {}", s.status);
                        control_catalog::select_page("QR codes");
                        stage.set(6);
                    }
                }
                6 => {
                    if smoke_snapshot().is_some_and(|s| s.title == "QR codes" && s.attached)
                        && smoke_action("decode")
                    {
                        stage.set(7);
                    }
                }
                7 => {
                    if let Some(s) =
                        smoke_snapshot().filter(|s| s.status.starts_with("Decoded 2 QR code(s):"))
                    {
                        println!("CATALOG_QR_IMAGE_PASS {}", s.status);
                        control_catalog::select_page("Buttons");
                        stage.set(8);
                    }
                }
                _ => {
                    println!("CATALOG_MEDIA_SMOKE_PASS camera_acquired=false");
                    if let Some(window) = super::main_window() {
                        window.close();
                    }
                    return false;
                }
            }
            true
        },
        Duration::from_millis(150),
        DispatcherPriority::BACKGROUND,
    )
    .detach();
}
