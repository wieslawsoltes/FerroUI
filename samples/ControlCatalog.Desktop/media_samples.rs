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
// The providers of the renderer in use: the shared desktop bindings, or the native Direct3D 11
// producers when the catalog is built with the Direct2D backend.
#[cfg_attr(all(windows, feature = "direct2d"), allow(dead_code))]
#[path = "../media_desktop.rs"]
mod desktop_providers;
#[cfg(not(all(windows, feature = "direct2d")))]
use desktop_providers::video;
#[cfg(all(windows, feature = "direct2d"))]
fn video(
    interop: &dyn ICompositionGpuInterop,
    budget: FrameBudget,
) -> Result<
    (
        Rc<dyn ferroui_media_video::MediaPlayerProvider>,
        Rc<dyn ferroui_media_controls::TextureFrameImporter>,
    ),
    ferroui_media::MediaError,
> {
    Ok((
        Rc::new(ferroui_media_video_windows::WindowsVideoProvider {
            frames: ferroui_media_windows::WindowsFrameProducer::from_direct2d(interop, budget)?,
        }),
        Rc::new(ferroui_direct2d::D3D11TextureImporter),
    ))
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
    #[cfg(all(windows, feature = "direct2d"))]
    {
        let frames = ferroui_media_windows::WindowsFrameProducer::from_direct2d(interop, budget).map_err(|error| {
            CameraError::new(ferroui_media_camera::CameraErrorKind::Unsupported, error.to_string())
        })?;
        let importer = Rc::new(ferroui_direct2d::D3D11TextureImporter);
        #[cfg(feature = "synthetic-camera")]
        if std::env::var("FERROUI_SMOKE_MEDIA_SYNTHETIC_CAMERA").as_deref() == Ok("1") {
            return Ok((Rc::new(ferroui_media_camera_windows::WindowsCameraProvider::synthetic(frames)), importer));
        }
        return Ok((Rc::new(ferroui_media_camera_windows::WindowsCameraProvider::new(frames)), importer));
    }
    #[cfg(all(windows, feature = "synthetic-camera", not(feature = "direct2d")))]
    if std::env::var("FERROUI_SMOKE_MEDIA_SYNTHETIC_CAMERA").as_deref() == Ok("1") {
        let frames = ferroui_media_opengl::NativeGlFrameProducer::from_interop(interop, budget)
            .map_err(|error| {
                CameraError::new(ferroui_media_camera::CameraErrorKind::Unsupported, error.to_string())
            })?;
        return Ok((
            Rc::new(ferroui_media_camera_windows::WindowsCameraProvider::synthetic(frames)),
            Rc::new(ferroui_media_windows::NativeGlTextureImporter),
        ));
    }
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
    #[cfg(not(all(windows, feature = "direct2d")))]
    desktop_providers::camera(interop, budget)
}

fn gpu_checkpoint(label: &str) {
    #[cfg(all(windows, feature = "direct2d"))]
    if std::env::var_os("FERROUI_D3D_STATS").is_some() {
        println!("CATALOG_GPU_CHECKPOINT {label} {:?}", ferroui_direct2d::statistics());
    }
    #[cfg(all(target_os = "macos", feature = "quartz-metal"))]
    if std::env::var_os("FERROUI_METAL_STATS").is_some() {
        println!(
            "CATALOG_GPU_CHECKPOINT {label} {:?}",
            ferroui_quartz::metal_statistics()
        );
    }
    let _ = label;
}

#[derive(Default)]
struct PlaybackCadence {
    first: Option<(std::time::Instant, u64)>,
    last: Option<(std::time::Instant, u64)>,
    frames: u64,
    max_gap: std::time::Duration,
}

impl PlaybackCadence {
    fn observe(&mut self, frame: &FrameMetadata) {
        let now = std::time::Instant::now();
        let sequence = frame.identity.sequence;
        if let Some((previous, last_sequence)) = self.last {
            if sequence <= last_sequence {
                return;
            }
            self.max_gap = self.max_gap.max(now - previous);
        }
        self.first.get_or_insert((now, sequence));
        self.last = Some((now, sequence));
        self.frames += 1;
    }

    fn sample(&self) -> Option<(u64, u64, f64, f64)> {
        let (started, first) = self.first?;
        let (ended, last) = self.last?;
        let elapsed = ended - started;
        (elapsed >= std::time::Duration::from_secs(3)).then(|| {
            (
                self.frames,
                last - first + 1,
                (self.frames - 1) as f64 / elapsed.as_secs_f64(),
                self.max_gap.as_secs_f64() * 1000.,
            )
        })
    }
}

pub static SMOKE_FAILED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn smoke() {
    if std::env::var("FERROUI_SMOKE_MEDIA").as_deref() != Ok("1") {
        return;
    }
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use std::{
        cell::{Cell, RefCell},
        time::{Duration, Instant},
    };
    let capture_camera = std::env::var("FERROUI_SMOKE_MEDIA_CAMERA").as_deref() == Ok("1");
    // Optional sustained delivery observation, after the normal warmup. Imports
    // count newly admitted video frames, independently of UI progress redraws.
    let video_observation_ms = std::env::var("FERROUI_SMOKE_MEDIA_VIDEO_MS")
        .ok().and_then(|value| value.parse::<u64>().ok()).unwrap_or(0).min(30000);
    let video_observation = Cell::new(None::<(Instant, u64)>);
    let synthetic_camera = cfg!(any(
        all(windows, feature = "synthetic-camera"),
        all(target_os = "macos", feature = "quartz-media-test-fixtures")
    )) && std::env::var("FERROUI_SMOKE_MEDIA_SYNTHETIC_CAMERA").as_deref() == Ok("1");
    let stage = Cell::new(0);
    let report_at = Cell::new(0u64);
    let began = Instant::now();
    let recording_started = Cell::new(None::<Instant>);
    let old = RefCell::new(None::<FrameBudget>);
    let video_page = std::env::var("FERROUI_SMOKE_MEDIA_PAGE").unwrap_or_else(|_| "Video".into());
    let check_cadence = std::env::var("FERROUI_SMOKE_MEDIA_CADENCE").as_deref() == Ok("1");
    let check_transport = std::env::var("FERROUI_SMOKE_MEDIA_TRANSPORT").as_deref() == Ok("1");
    let transport_baseline = Cell::new(None::<(Instant, Duration)>);
    let cadence = Rc::new(RefCell::new(PlaybackCadence::default()));
    let cadence_subscription = RefCell::new(None);
    DispatcherTimer::run(
        move || {
            use control_catalog::media_samples::{
                smoke_action, smoke_capture, smoke_on_frame_presented, smoke_snapshot,
                smoke_transport_item, smoke_video_element,
            };
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
                    if let Some(window) = super::main_window() { window.set_title(Some("Automated media smoke".into())); }
                    if control_catalog::select_page(&video_page) {
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
                        if check_cadence {
                            let cadence = cadence.clone();
                            let warmup = Instant::now() + Duration::from_millis(500);
                            cadence_subscription.replace(smoke_on_frame_presented(move |frame| {
                                if Instant::now() >= warmup {
                                    cadence.borrow_mut().observe(frame);
                                }
                            }));
                        }
                        smoke_action("play");
                        stage.set(3);
                    }
                }
                3 => {
                    if let Some(s) = smoke_snapshot().filter(|s| {
                        s.playing && s.frame && s.budget.counters().snapshot().gpu_conversions >= 5
                    }) {
                        if check_cadence {
                            let Some((frames, source_frames, fps, max_gap_ms)) = cadence.borrow().sample()
                            else {
                                return true;
                            };
                            println!(
                                "CATALOG_VIDEO_CADENCE frames={frames} source_frames={source_frames} fps={fps:.1} max_gap_ms={max_gap_ms:.1}"
                            );
                            // The bundled 24 fps clip used to pass this smoke
                            // after presenting only a handful of frames.
                            if cfg!(windows)
                                && (fps < 20. || frames * 100 < source_frames * 85 || max_gap_ms > 160.)
                            {
                                fail("Windows playback missed the cadence target for the bundled clip");
                                return false;
                            }
                            cadence_subscription.borrow_mut().take();
                        }
                        let counts=s.budget.counters().snapshot();
                        if counts.preview != PixelTransferCount::default() || counts.analysis != PixelTransferCount::default() || counts.photo != PixelTransferCount::default() || counts.diagnostic != PixelTransferCount::default() || counts.probe != PixelTransferCount::default() {
                            fail("Video CPU pixel transfer");
                            return false;
                        }
                        if video_observation_ms > 0 {
                            let (start, imports) = video_observation.get().unwrap_or_else(|| {
                                let observation = (Instant::now(), counts.gpu_imports);
                                video_observation.set(Some(observation));
                                observation
                            });
                            let elapsed = start.elapsed();
                            if elapsed < Duration::from_millis(video_observation_ms) { return true; }
                            let delivered = counts.gpu_imports - imports;
                            println!("CATALOG_VIDEO_DELIVERY frames={delivered} elapsed_ms={} fps={:.2}", elapsed.as_millis(), delivered as f64 / elapsed.as_secs_f64());
                        }
                        gpu_checkpoint("video");
                        println!("CATALOG_VIDEO_PASS autoplay=false shared_texture=true counters={:?}",s.budget.counters().snapshot());
                        if check_transport {
                            if !smoke_transport_item(&["Playback speed", "1.5×"]) {
                                fail("Playback speed action unavailable");
                                return false;
                            }
                            stage.set(30);
                        } else {
                            control_catalog::select_page("Camera");
                            stage.set(4);
                        }
                    }
                }
                30..=32 => {
                    let Some(view) = smoke_video_element() else { return true };
                    let Some(player) = view.media_player() else { return true };
                    let position = player.playback_session().position;
                    let (then, original) = transport_baseline.get().unwrap_or_else(|| {
                        let value = (Instant::now(), position);
                        transport_baseline.set(Some(value));
                        value
                    });
                    match stage.get() {
                        30 if then.elapsed() >= Duration::from_secs(2) => {
                            let rate = position.saturating_sub(original).as_secs_f64() / then.elapsed().as_secs_f64();
                            if !(1.25..=1.75).contains(&rate) || player.pause().is_err()
                                || !smoke_transport_item(&["Playback speed", "1×"])
                                || !smoke_transport_item(&["Skip forward 30 seconds"]) {
                                fail(&format!("Playback rate or forward action failed, measured rate {rate:.2}"));
                                return false;
                            }
                            println!("CATALOG_PLAYBACK_RATE_PASS measured={rate:.2}");
                            transport_baseline.set(Some((Instant::now(), position)));
                            stage.set(31);
                        }
                        31 if position + Duration::from_millis(500) >= player.playback_session().duration
                            .map_or(original + Duration::from_secs(30), |end| end.min(original + Duration::from_secs(30))) => {
                            if !smoke_transport_item(&["Skip back 10 seconds"]) {
                                fail("Backward action unavailable");
                                return false;
                            }
                            transport_baseline.set(Some((Instant::now(), position)));
                            stage.set(32);
                        }
                        32 if position + Duration::from_secs(9) <= original => {
                            let stretch = view.presenter().stretch();
                            if !smoke_transport_item(&["Zoom to fill"])
                                || view.presenter().stretch() != ferroui_base::media::Stretch::UniformToFill
                                || !smoke_transport_item(&["Fit video"]) || view.presenter().stretch() != stretch {
                                fail("Zoom action did not change and restore the video stretch");
                                return false;
                            }
                            println!("CATALOG_TRANSPORT_PASS skip and zoom");
                            control_catalog::select_page("Camera");
                            stage.set(4);
                        }
                        _ => {}
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
                        println!("CATALOG_CAMERA_CAPTURE_PASS synthetic={synthetic_camera} counters={counts:?}");
                        old.replace(Some(s.budget));
                        control_catalog::select_page("Buttons");
                        stage.set(21);
                    }
                }
                21 => {
                    if old.borrow().as_ref().is_some_and(|b| b.usage().frames == 0 && b.gpu_reads() == 0 && b.quarantined_reads() == 0) {
                        println!("CATALOG_CAMERA_NAVIGATION_RETIREMENT_PASS");
                        control_catalog::select_page("CameraCaptureUI");
                        stage.set(22);
                    }
                }
                22 => {
                    if let Some(capture) = smoke_capture() {
                        capture.control.start_preview();
                        stage.set(23);
                    }
                }
                23..=27 => {
                    let Some(capture) = smoke_capture() else { return true };
                    if let Some(error) = capture.error.take() {
                        if stage.get() == 24 && error == ferroui_media_camera::CameraErrorKind::Busy {
                            capture.control.take_photo();
                            return true;
                        }
                        fail(&format!("CameraCaptureUI stage={} {error:?}", stage.get()));
                        return false;
                    }
                    match stage.get() {
                        23 if capture.budget.counters().snapshot().gpu_conversions >= 5 => {
                            capture.control.take_photo();
                            stage.set(24);
                        }
                        24 if capture.photo.source().is_some() => {
                            if capture.control.preview().and_then(|p| p.session())
                                .is_some_and(|s| s.supports_video_recording())
                            {
                                capture.control.start_recording();
                                stage.set(25);
                            } else {
                                if cfg!(windows) && synthetic_camera {
                                    fail("The Media Foundation fixture did not advertise H.264 recording");
                                    return false;
                                }
                                capture.control.cancel();
                                stage.set(27);
                            }
                        }
                        25 if !capture.control.is_busy() => {
                            let start = recording_started.get().unwrap_or_else(|| {
                                let now = Instant::now();
                                recording_started.set(Some(now));
                                now
                            });
                            if start.elapsed() >= Duration::from_secs(1) {
                                capture.control.finish_recording();
                                stage.set(26);
                            }
                        }
                        26 if capture.movie_valid.get() => {
                            println!("CATALOG_CAMERA_UI_RECORDING_PASS synthetic={synthetic_camera}");
                            capture.control.cancel();
                            stage.set(27);
                        }
                        27 if capture.budget.usage().frames == 0
                            && capture.budget.gpu_reads() == 0
                            && capture.budget.quarantined_reads() == 0 =>
                        {
                            if capture.photo.source().is_none()
                                || capture.budget.counters().snapshot().preview != PixelTransferCount::default()
                            {
                                fail("Photo ownership or preview transfer check failed");
                                return false;
                            }
                            println!("CATALOG_CAMERA_UI_PHOTO_PASS synthetic={synthetic_camera} retained_after_stop=true");
                            control_catalog::select_page("QR codes");
                            stage.set(6);
                        }
                        _ => {}
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
