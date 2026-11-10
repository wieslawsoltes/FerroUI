//! Desktop catalog bindings to admitted native media providers.
use control_catalog::media_samples::MediaSampleProviders;
use ferroui_base::rendering::composition::{frames::*, ICompositionGpuInterop};
use ferroui_media_camera::CameraError;
#[cfg(all(target_os = "macos", feature = "quartz-media-test-fixtures"))]
use ferroui_media_camera::CameraErrorKind;
use std::rc::Rc;

#[cfg(all(target_os = "macos", feature = "quartz-media-test-fixtures"))]
#[path = "media_camera_fixture.rs"]
mod camera_fixture;

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
#[path = "../media_desktop.rs"]
mod desktop_providers;
use desktop_providers::video;
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
    #[cfg(all(target_os = "macos", feature = "quartz-media-test-fixtures"))]
    if std::env::var("FERROUI_SMOKE_MEDIA_SYNTHETIC_CAMERA").as_deref() == Ok("1") {
        let device = DeviceIdentity::Luid(interop.device_luid().ok_or_else(|| {
            CameraError::new(CameraErrorKind::Unsupported, "No admitted Metal device")
        })?);
        return Ok((
            Rc::new(camera_fixture::Provider {
                device,
                generation: interop.shared_context_generation(),
            }),
            Rc::new(ferroui_media_apple::AppleTextureImporter),
        ));
    }
    desktop_providers::camera(interop, budget)
}

fn gpu_checkpoint(label: &str) {
    #[cfg(all(target_os = "macos", feature = "quartz-metal"))]
    if std::env::var_os("FERROUI_METAL_STATS").is_some() {
        println!(
            "CATALOG_GPU_CHECKPOINT {label} {:?}",
            ferroui_quartz::metal_statistics()
        );
    }
    let _ = label;
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
    let capture_camera = std::env::var("FERROUI_SMOKE_MEDIA_CAMERA").as_deref() == Ok("1");
    let stage = Cell::new(0);
    let report_at = Cell::new(0u64);
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
            if began.elapsed().as_secs() >= report_at.get() {
                if let Some(s) = smoke_snapshot() {
                    println!("CATALOG_MEDIA_PROGRESS stage={} title={} paused={} playing={} frame={} camera={} status={:?} counters={:?}",stage.get(),s.title,s.paused,s.playing,s.frame,s.camera_active,s.status,s.budget.counters().snapshot());
                }
                report_at.set(began.elapsed().as_secs()+5);
            }
            match stage.get() {
                0 => {
                    gpu_checkpoint("start");
                    if let Some(window) = super::main_window() { window.set_title(Some("AUTOMATED MEDIA TEST — Quartz Metal".into())); }
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
                        s.playing && s.frame && s.budget.counters().snapshot().gpu_conversions >= 5
                    }) {
                        let counts=s.budget.counters().snapshot();
                        if counts.preview != PixelTransferCount::default() || counts.analysis != PixelTransferCount::default() || counts.photo != PixelTransferCount::default() || counts.diagnostic != PixelTransferCount::default() || counts.probe != PixelTransferCount::default() {
                            fail("Video CPU pixel transfer");
                            return false;
                        }
                        gpu_checkpoint("video");
                        println!("CATALOG_VIDEO_PASS autoplay=false shared_texture=true counters={:?}",s.budget.counters().snapshot());
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
                        if capture_camera {
                            smoke_action("start_camera");
                            stage.set(20);
                        } else {
                            control_catalog::select_page("QR codes");
                            stage.set(6);
                        }
                    }
                }
                20 => {
                    if let Some(s) = smoke_snapshot().filter(|s| s.title == "Camera" && s.camera_active && s.frame && s.budget.counters().snapshot().gpu_conversions >= 5) {
                        let counts = s.budget.counters().snapshot();
                        if counts.preview != PixelTransferCount::default() || counts.analysis != PixelTransferCount::default() || counts.photo != PixelTransferCount::default() || counts.diagnostic != PixelTransferCount::default() || counts.probe != PixelTransferCount::default() {
                            fail("Camera CPU pixel transfer"); return false;
                        }
                        gpu_checkpoint("camera");
                        println!("CATALOG_CAMERA_CAPTURE_PASS {} synthetic={} counters={:?}",s.status,std::env::var("FERROUI_SMOKE_MEDIA_SYNTHETIC_CAMERA").as_deref()==Ok("1"),counts);
                        old.replace(Some(s.budget));
                        control_catalog::select_page("Buttons");
                        stage.set(21);
                    }
                }
                21 => {
                    if old.borrow().as_ref().is_some_and(|b| b.usage().frames == 0 && b.gpu_reads() == 0 && b.quarantined_reads() == 0) {
                        println!("CATALOG_CAMERA_NAVIGATION_RETIREMENT_PASS");
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
                    println!("CATALOG_MEDIA_SMOKE_PASS camera_acquired={capture_camera}");
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
