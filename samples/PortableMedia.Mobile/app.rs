#[path = "scene.rs"]
mod scene;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate,
    rendering::composition::{frames::*, ElementComposition},
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
    FerroObjectImpl, Ref,
};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Control, NewApplication, Panel,
};
#[cfg(target_os = "android")]
use ferroui_media_android::{NativeGlFrameProducer, NativeGlTextureImporter};
use ferroui_media_video::{MediaPlaybackState, MediaSource};
use ferroui_media_video_controls::MediaPlayerElement;
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};

#[repr(C)]
pub struct App {
    base: Application,
}
ferro_class!(App:Application);
ferro_impl_classes!(App:FerroObjectImpl);
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        ferroui_media_video_controls::register_types();
        ferroui_media_camera_controls::register_types();
        this.styles()
            .add(ferroui_themes_winui::WinUITheme::new().as_style());
    }
    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        #[cfg(target_os = "ios")]
        if let Some(lifetime) = lifetime
            .as_ref()
            .and_then(|v| v.as_single_view_application_lifetime())
        {
            lifetime.set_main_view(Some(create()));
        }
        #[cfg(target_os = "android")]
        if let Some(lifetime) = lifetime
            .as_ref()
            .and_then(|v| v.as_activity_application_lifetime())
        {
            lifetime.set_main_view_factory(Some(Rc::new(create)));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}
#[cfg(target_os = "ios")]
pub struct AppDelegate;
#[cfg(target_os = "ios")]
impl ferroui_ios::FerroApplicationDelegate for AppDelegate {
    fn create_app_builder(&self, delegate: Rc<dyn ferroui_ios::IFerroAppDelegate>) -> AppBuilder {
        use ferroui_ios::IosApplicationExtensions;
        AppBuilder::configure::<App>().use_ios_with_delegate(delegate)
    }
}
#[cfg(target_os = "android")]
pub fn build() -> AppBuilder {
    use ferroui_android::AndroidApplicationExtensions;
    AppBuilder::configure::<App>().use_android()
}
fn launch_flag(ios: &str, android: &str) -> bool {
    #[cfg(target_os = "ios")]
    {
        let _ = android;
        std::env::args().any(|v| v == ios)
    }
    #[cfg(target_os = "android")]
    {
        let _ = ios;
        use ferroui_android::{interop::java::*, FerroActivity};
        FerroActivity::current_main_activity()
            .and_then(|v| {
                call_object(
                    v.java_object(),
                    "getIntent",
                    "()Landroid/content/Intent;",
                    &[],
                )
            })
            .is_some_and(|v| {
                call_boolean(
                    &v,
                    "getBooleanExtra",
                    "(Ljava/lang/String;Z)Z",
                    &[JavaValue::String(android), JavaValue::Boolean(false)],
                )
            })
    }
}
fn is_smoke() -> bool {
    launch_flag("--smoke", "mediaSmoke")
}
fn device_recovery_smoke() -> bool {
    launch_flag("--device-recovery-smoke", "deviceRecoverySmoke")
}
fn saturation_smoke() -> bool {
    launch_flag("--saturation-smoke", "saturationSmoke")
}
fn renderer_smoke() -> bool {
    launch_flag("--renderer-smoke", "rendererSmoke")
}
fn crop_smoke() -> bool {
    launch_flag("--crop-smoke", "cropSmoke")
}
fn geometry_smoke() -> bool {
    launch_flag("--geometry-smoke", "geometrySmoke")
}
#[cfg(target_os = "android")]
fn declared_camera_color() -> Option<FrameColor> {
    use ferroui_android::{interop::java::*, FerroActivity};
    let declared = FerroActivity::current_main_activity()
        .and_then(|v| {
            call_object(
                v.java_object(),
                "getIntent",
                "()Landroid/content/Intent;",
                &[],
            )
        })
        .is_some_and(|v| {
            call_boolean(
                &v,
                "getBooleanExtra",
                "(Ljava/lang/String;Z)Z",
                &[
                    JavaValue::String("cameraSdrSrgb"),
                    JavaValue::Boolean(false),
                ],
            )
        });
    if declared {
        report(
            "CAMERA_PROFILE untagged source explicitly declared sRGB; native tags retain priority",
        );
        Some(FrameColor::Srgb)
    } else {
        None
    }
}
static SMOKE_FAILED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static RENDERER_PROBE_PASSES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
fn report(value: &str) {
    use std::sync::atomic::Ordering;
    let tag = value.split_whitespace().next().unwrap_or("");
    if value.starts_with("RESULT: FAIL")
        || value.starts_with("NATIVE_QR FAIL")
        || tag.ends_with("_FAIL")
        || tag.ends_with("_ERROR")
    {
        SMOKE_FAILED.store(true, Ordering::SeqCst);
    }
    if value.starts_with("RENDERER_PIXELS_PASS") {
        RENDERER_PROBE_PASSES.fetch_add(1, Ordering::SeqCst);
    }
    let value = if value.starts_with("RESULT: PASS") && SMOKE_FAILED.load(Ordering::SeqCst) {
        "RESULT: FAIL an earlier smoke check failed; cleanup completed"
    } else {
        value
    };
    println!("{value}");
    #[cfg(target_os = "ios")]
    let directory = std::path::PathBuf::from(std::env::var("HOME").unwrap()).join("Documents");
    #[cfg(target_os = "android")]
    let directory = {
        use ferroui_android::{interop::java::*, FerroAndroidApplication};
        let file = call_object(
            &FerroAndroidApplication::context(),
            "getFilesDir",
            "()Ljava/io/File;",
            &[],
        )
        .unwrap();
        std::path::PathBuf::from(string_of(
            &call_object(&file, "getAbsolutePath", "()Ljava/lang/String;", &[]).unwrap(),
        ))
    };
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("portable-media-report.txt"))
    {
        let _ = writeln!(file, "{value}");
    }
}
fn create() -> Ref<Control> {
    let root = Panel::new();
    let view = MediaPlayerElement::new();
    root.children().add(&view);
    view.set_are_transport_controls_enabled(true);
    let weak = view.downgrade();
    let weak_root = root.downgrade();
    let started = Instant::now();
    let phase = Rc::new(Cell::new(0));
    DispatcherTimer::run(
        move || {
            if phase.get() == 2 {
                return false;
            }
            if started.elapsed() > Duration::from_secs(20) {
                report("RESULT: FAIL GPU initialization timed out");
                return false;
            }
            if phase.get() == 1 {
                return true;
            }
            let Some(view) = weak.upgrade() else {
                return false;
            };
            let Some(visual) = ElementComposition::get_element_visual(&view) else {
                if started.elapsed() > Duration::from_secs(15) {
                    report("RESULT: FAIL no compositor");
                    return false;
                }
                return true;
            };
            let compositor = visual.compositor().clone();
            // Android creates the view before onResume establishes its current
            // activity. Read launch options when the compositor is ready.
            let smoke = is_smoke();
            view.set_autoplay(smoke || renderer_smoke());
            let weak = view.downgrade();
            let weak_root = weak_root.clone();
            let phase = phase.clone();
            phase.set(1);
            Dispatcher::ui_thread().spawn_local(async move {
                let Some(interop) = compositor.try_get_composition_gpu_interop().await else {
                    phase.set(0);
                    return;
                };
                phase.set(2);
                if smoke {
                    native_qr_smoke().await;
                }
                let Some(view) = weak.upgrade() else {
                    return;
                };
                #[cfg(target_os = "android")]
                if smoke {
                    if let Some(root) = weak_root.upgrade() {
                        if let Err(error) = prove_gpu(&root, &*interop).await {
                            report(&format!("RESULT: FAIL independent GPU {error}"));
                            return;
                        }
                    }
                }
                if let Some(root) = weak_root.upgrade() {
                    camera_actions(&root, &view, &*interop);
                }
                let budget = FrameBudget::new(FrameBudgetLimits::default());
                #[cfg(target_os = "ios")]
                let (provider, importer) = {
                    use ferroui_media_video_apple::{AppleTextureImporter, AppleVideoProvider};
                    let Some(device) = interop.device_luid() else {
                        report("RESULT: FAIL no Metal device");
                        return;
                    };
                    (
                        Rc::new(AppleVideoProvider {
                            device: DeviceIdentity::Luid(device),
                            device_generation: interop.shared_context_generation(),
                            budget: budget.clone(),
                        })
                            as Rc<dyn ferroui_media_video::MediaPlayerProvider>,
                        Rc::new(AppleTextureImporter)
                            as Rc<dyn ferroui_media_controls::TextureFrameImporter>,
                    )
                };
                #[cfg(target_os = "android")]
                let (provider, importer) = {
                    use ferroui_media_android::{NativeGlFrameProducer, NativeGlTextureImporter};
                    let frames =
                        match NativeGlFrameProducer::from_interop(&*interop, budget.clone()) {
                            Ok(v) => v,
                            Err(e) => {
                                report(&format!("RESULT: FAIL {e}"));
                                return;
                            }
                        };
                    (
                        Rc::new(ferroui_media_video_android::AndroidVideoProvider { frames })
                            as Rc<dyn ferroui_media_video::MediaPlayerProvider>,
                        Rc::new(NativeGlTextureImporter)
                            as Rc<dyn ferroui_media_controls::TextureFrameImporter>,
                    )
                };
                let source = MediaSource::Resource {
                    name: "avc-aac.mp4".into(),
                    content_type: "video/mp4".into(),
                    encoded: if crop_smoke() {
                        include_bytes!("../../tests/media/fixtures/avc-aac-geometry-crop.mp4")
                            .as_slice()
                            .into()
                    } else if geometry_smoke() {
                        include_bytes!("../../tests/media/fixtures/avc-aac-rotate90-mirror.mp4")
                            .as_slice()
                            .into()
                    } else {
                        include_bytes!("../../tests/media/fixtures/avc-aac.mp4")
                            .as_slice()
                            .into()
                    },
                };
                let pending = view.set_source(source, provider, importer);
                let recovery_budget = budget.clone();
                view.set_provider_factory(Some(Rc::new(move |interop| {
                    #[cfg(target_os = "ios")]
                    {
                        use ferroui_media_video_apple::{AppleTextureImporter, AppleVideoProvider};
                        Ok((
                            Rc::new(AppleVideoProvider {
                                device: DeviceIdentity::Luid(interop.device_luid().ok_or_else(
                                    || {
                                        ferroui_media::MediaError::new(
                                            ferroui_media::MediaErrorKind::Unsupported,
                                            "No restored Metal device",
                                        )
                                    },
                                )?),
                                device_generation: interop.shared_context_generation(),
                                budget: recovery_budget.clone(),
                            }),
                            Rc::new(AppleTextureImporter),
                        ))
                    }
                    #[cfg(target_os = "android")]
                    {
                        Ok((
                            Rc::new(ferroui_media_video_android::AndroidVideoProvider {
                                frames: NativeGlFrameProducer::from_interop(
                                    interop,
                                    recovery_budget.clone(),
                                )?,
                            }),
                            Rc::new(NativeGlTextureImporter),
                        ))
                    }
                })));
                match pending.await {
                    Err(e) => report(&format!("RESULT: FAIL open {e}")),
                    Ok(()) => {
                        report("OPENED AVC/AAC resource native audio/timing");
                        if smoke {
                            start_smoke(&view, budget);
                        }
                    }
                }
            });
            true
        },
        Duration::from_millis(50),
        DispatcherPriority::NORMAL,
    );
    let host = ferroui_controls::UserControl::new();
    host.set_content(Some(Control::boxed(root)));
    ferroui_controls::TopLevel::set_auto_safe_area_padding(&host, true);
    host.upcast()
}

#[cfg(target_os = "android")]
async fn prove_gpu(
    root: &Ref<Panel>,
    interop: &dyn ferroui_base::rendering::composition::ICompositionGpuInterop,
) -> Result<(), ferroui_media::MediaError> {
    use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
    use ferroui_controls::StackPanel;
    use ferroui_media_android::{
        gpu_quadrants, native_error, poll_native, NativeGlFrameProducer, NativeGlTextureImporter,
    };
    use ferroui_media_controls::TexturePresenter;
    use std::{sync::Arc, task::Poll};
    let budget = FrameBudget::new(Default::default());
    let producer = NativeGlFrameProducer::from_interop(interop, budget.clone())?;
    let submitted = gpu_quadrants(
        &producer,
        FrameIdentity {
            source: 0xD00D,
            generation: 1,
            sequence: 1,
        },
        32,
        32,
    )
    .await?
    .ok_or_else(|| native_error("Independent producer saturated"))?;
    let frame = poll_native(move || match submitted.take_if_ready() {
        Ok(Some(frame)) => Poll::Ready(Ok(frame)),
        Ok(None) => Poll::Pending,
        Err(e) => Poll::Ready(Err(e)),
    })
    .await?;
    let mailbox = Arc::new(FrameMailbox::new(0xD00D, 1));
    mailbox
        .publish(frame.clone())
        .map_err(|e| native_error(e.to_string()))?;
    let host = StackPanel::new();
    host.set_orientation(Orientation::Horizontal);
    host.set_horizontal_alignment(HorizontalAlignment::Center);
    host.set_vertical_alignment(VerticalAlignment::Center);
    let mut views = Vec::new();
    for _ in 0..2 {
        let view = TexturePresenter::new();
        view.set_width(96.);
        view.set_height(96.);
        view.set_frame_source(
            Some(mailbox.clone()),
            Some(Rc::new(NativeGlTextureImporter)),
        );
        host.children().add(&view);
        views.push(view);
    }
    root.children().add(&host);
    let counters = budget.counters();
    let observed = counters.clone();
    let started = Instant::now();
    let result = poll_native(move || {
        if observed.snapshot().gpu_imports >= 2 {
            return Poll::Ready(Ok(()));
        }
        if started.elapsed() > Duration::from_secs(10) {
            return Poll::Ready(Err(native_error(
                "Independent frame was not sampled by both views",
            )));
        }
        Poll::Pending
    })
    .await;
    report(&format!(
        "INDEPENDENT_GPU_TWO_VIEWS {:?}",
        counters.snapshot()
    ));
    for view in views {
        view.set_frame_source(None, None);
    }
    root.children().remove(&host);
    drop(host);
    drop(frame);
    drop(mailbox);
    result?;
    let started = Instant::now();
    poll_native(move || {
        if budget.usage().frames == 0 {
            return Poll::Ready(Ok(()));
        }
        if started.elapsed() > Duration::from_secs(5) {
            return Poll::Ready(Err(native_error(
                "Independent GPU storage failed to retire",
            )));
        }
        Poll::Pending
    })
    .await?;
    report("INDEPENDENT_GPU_RETIRED frames=0");
    Ok(())
}

fn camera_actions(
    root: &Ref<Panel>,
    video: &Ref<MediaPlayerElement>,
    interop: &dyn ferroui_base::rendering::composition::ICompositionGpuInterop,
) {
    use ferroui_base::{
        layout::{Orientation, VerticalAlignment},
        BoxedValue,
    };
    use ferroui_controls::{automation::AutomationProperties, Button, StackPanel};
    use ferroui_media_camera::{CameraCapture, CameraProvider};
    use ferroui_media_camera_controls::CameraPreview;
    let budget = FrameBudget::new(Default::default());
    #[cfg(target_os = "ios")]
    let (provider, importer) = {
        let Some(device) = interop.device_luid() else {
            return;
        };
        (
            Rc::new(ferroui_media_camera_apple::AppleCameraProvider {
                device: DeviceIdentity::Luid(device),
                device_generation: interop.shared_context_generation(),
            }) as Rc<dyn CameraProvider>,
            Rc::new(ferroui_media_camera_apple::AppleTextureImporter)
                as Rc<dyn ferroui_media_controls::TextureFrameImporter>,
        )
    };
    #[cfg(target_os = "android")]
    let (provider, importer) = {
        let Ok(frames) =
            ferroui_media_android::NativeGlFrameProducer::from_interop(interop, budget.clone())
        else {
            return;
        };
        (
            Rc::new(ferroui_media_camera_android::AndroidCameraProvider {
                frames: Some(frames),
                untagged_sdr_color: declared_camera_color(),
            }) as Rc<dyn CameraProvider>,
            Rc::new(ferroui_media_android::NativeGlTextureImporter)
                as Rc<dyn ferroui_media_controls::TextureFrameImporter>,
        )
    };
    let capture = CameraCapture::new(provider, budget.clone());
    let devices = capture.devices();
    Dispatcher::ui_thread().spawn_local(async move {
        match devices.await {
            Ok(devices) => report(&format!(
                "CAMERA_DISCOVERY devices={} no acquisition or prompt",
                devices.len()
            )),
            Err(error) => report(&format!("CAMERA_DISCOVERY_ERROR {error}")),
        }
    });
    let camera = CameraPreview::new();
    camera.set_is_visible(false);
    root.children().add(&camera);
    let recovery_budget = budget.clone();
    camera.set_provider_factory(Some(Rc::new(move |interop| {
        use ferroui_media_camera::{CameraError, CameraErrorKind};
        #[cfg(target_os = "ios")]
        {
            let _ = &recovery_budget;
            Ok((
                Rc::new(ferroui_media_camera_apple::AppleCameraProvider {
                    device: DeviceIdentity::Luid(interop.device_luid().ok_or_else(|| {
                        CameraError::new(
                            CameraErrorKind::Unsupported,
                            "No restored camera Metal device",
                        )
                    })?),
                    device_generation: interop.shared_context_generation(),
                }),
                Rc::new(ferroui_media_camera_apple::AppleTextureImporter),
            ))
        }
        #[cfg(target_os = "android")]
        {
            let frames = NativeGlFrameProducer::from_interop(interop, recovery_budget.clone())
                .map_err(|e| CameraError::new(CameraErrorKind::Unsupported, e.to_string()))?;
            Ok((
                Rc::new(ferroui_media_camera_android::AndroidCameraProvider {
                    frames: Some(frames),
                    untagged_sdr_color: declared_camera_color(),
                }),
                Rc::new(NativeGlTextureImporter),
            ))
        }
    })));

    let actions = StackPanel::new();
    actions.set_orientation(Orientation::Horizontal);
    actions.set_vertical_alignment(VerticalAlignment::Top);
    actions.set_spacing(8.);
    let button = |label: &str| {
        let value = Button::new();
        value.set_content(Some(Rc::new(label.to_owned()) as BoxedValue));
        AutomationProperties::set_name(&value, Some(label));
        value
    };
    let start = button("Camera");
    let stop = button("Stop");
    let photo = button("JPEG");
    let navigate = button("Detach");
    let weak = camera.downgrade();
    let video_weak = video.downgrade();
    start.click(move |_, _| {
        if let Some(video) = video_weak.upgrade() {
            if let Some(player) = video.media_player() {
                player.clear_source();
            }
            video.set_is_visible(false);
        }
        let Some(camera) = weak.upgrade() else {
            return;
        };
        camera.set_is_visible(true);
        let mut options = ferroui_media_camera::CameraStartOptions::default();
        options.selection.format = Some(ferroui_media_camera::CameraFormat {
            width: 1280,
            height: 720,
            frames_per_second: 30.,
        });
        let pending = camera.start(capture.clone(), options, importer.clone());
        Dispatcher::ui_thread().spawn_local(async move {
            match pending.await {
                Ok(()) => report("CAMERA_STARTED"),
                Err(error) => report(&format!("CAMERA_START_ERROR {:?} {error}", error.kind)),
            }
        });
    });
    let weak = camera.downgrade();
    let retirement_budget = budget.clone();
    stop.click(move |_, _| {
        if let Some(camera) = weak.upgrade() {
            camera.stop();
            report("CAMERA_STOPPED");
            watch_camera_retirement(camera.downgrade(), retirement_budget.clone());
        }
    });
    let weak = camera.downgrade();
    photo.click(move |_, _| {
        let Some(session) = weak.upgrade().and_then(|v| v.session()) else {
            return;
        };
        let pending = session.photo();
        Dispatcher::ui_thread().spawn_local(async move {
            match pending.await {
                Ok(photo) => match image::load_from_memory_with_format(
                    &photo.encoded,
                    image::ImageFormat::Jpeg,
                ) {
                    Ok(decoded) => report(&format!(
                        "CAMERA_JPEG decoded={}x{} native={}x{} rotation={} origin={:?}",
                        decoded.width(),
                        decoded.height(),
                        photo.width,
                        photo.height,
                        photo.rotation_degrees,
                        photo.origin
                    )),
                    Err(error) => report(&format!("CAMERA_JPEG_DECODE_ERROR {error}")),
                },
                Err(error) if error.kind == ferroui_media_camera::CameraErrorKind::Cancelled => {
                    report("CAMERA_PHOTO_CANCELLED")
                }
                Err(error) => report(&format!("CAMERA_PHOTO_ERROR {error}")),
            }
        });
    });
    let root_weak = root.downgrade();
    let weak = camera.downgrade();
    let retirement_budget = budget.clone();
    navigate.click(move |_, _| {
        if let (Some(root), Some(camera)) = (root_weak.upgrade(), weak.upgrade()) {
            root.children().remove(&camera);
            report("CAMERA_PREVIEW_DETACHED");
            watch_camera_retirement(camera.downgrade(), retirement_budget.clone());
        }
    });
    actions.children().add(start);
    actions.children().add(stop);
    actions.children().add(photo);
    actions.children().add(navigate);
    qr_action(&actions, &camera);
    let reset = button("Reset GPU");
    let weak = camera.downgrade();
    reset.click(move |_, _| {
        if let Some(camera) = weak.upgrade() {
            if let Some(visual) = ElementComposition::get_element_visual(&camera) {
                report("GPU_RESET_REQUESTED");
                visual.compositor().invoke_server_job_async(
                    |server| {
                        server.reset_all_gpu_resources();
                        server.render_interface().value();
                        Ok(())
                    },
                    false,
                );
            }
        }
    });
    actions.children().add(reset);

    renderer_action(root, &actions, &camera, video, interop);
    root.children().add(actions);
}
fn watch_camera_retirement(
    camera: ferroui_base::WeakRef<ferroui_media_camera_controls::CameraPreview>,
    budget: FrameBudget,
) {
    let started = Instant::now();
    DispatcherTimer::run(
        move || {
            if camera.upgrade().and_then(|v| v.session()).is_some() {
                return false;
            }
            let usage = budget.usage();
            if usage.frames == 0 {
                report(&format!(
                    "CAMERA_RETIRED frames=0 gpu_reads={} quarantined_reads={}",
                    budget.gpu_reads(),
                    budget.quarantined_reads()
                ));
                return false;
            }
            if started.elapsed() > Duration::from_secs(5) {
                report(&format!(
                    "CAMERA_RETIREMENT_FAIL retained={usage:?} gpu_reads={} quarantined_reads={}",
                    budget.gpu_reads(),
                    budget.quarantined_reads()
                ));
                return false;
            }
            true
        },
        Duration::from_millis(16),
        DispatcherPriority::NORMAL,
    );
}

fn start_smoke(view: &Ref<MediaPlayerElement>, budget: FrameBudget) {
    let saturation = saturation_smoke();
    let recovery_started = Cell::new(false);
    let previous_generation = Cell::new(0u64);
    let previous_position = Cell::new(Duration::ZERO);
    let held = std::cell::RefCell::new(Vec::<FrameLease>::new());
    let released = Cell::new(false);
    let checked = Cell::new(false);
    let weak = view.downgrade();
    let stage = Rc::new(Cell::new(0));
    let changed = Cell::new(Instant::now());
    let started = Instant::now();
    DispatcherTimer::run(
        move || {
            let Some(view) = weak.upgrade() else {
                return false;
            };
            if started.elapsed() > Duration::from_secs(30) {
                #[cfg(target_os = "android")]
                report(&ferroui_media_video_android::native_diagnostics());
                report(&format!(
                    "RESULT: FAIL timeout stage={} retained={:?} gpu_reads={} quarantined_reads={}",
                    stage.get(),
                    budget.usage(),
                    budget.gpu_reads(),
                    budget.quarantined_reads()
                ));
                return false;
            }
            if stage.get() == 6 && budget.usage().frames == 0 {
                #[cfg(target_os = "android")]
                if ferroui_media_android::live_surface_sources() != 0
                    || ferroui_media_video_android::live_native_players() != 0
                {
                    return true;
                }
                if renderer_smoke()
                    && RENDERER_PROBE_PASSES.load(std::sync::atomic::Ordering::SeqCst)
                        < if device_recovery_smoke() { 2 } else { 1 }
                    && !SMOKE_FAILED.load(std::sync::atomic::Ordering::SeqCst)
                {
                    return true;
                }
                report("RESULT: PASS playback pause seek resume stop frame-zero detach retained=0");
                return false;
            }
            let Some(player) = view.media_player() else {
                return true;
            };
            let state = player.playback_session();
            if state.state == MediaPlaybackState::Failed && stage.get() != 8 {
                report(&format!("RESULT: FAIL {:?}", state.error));
                return false;
            }
            if saturation && stage.get() == 0 {
                if let Some(frame) = player.frames().latest() {
                    if !held
                        .borrow()
                        .iter()
                        .any(|v| v.metadata().identity == frame.metadata().identity)
                    {
                        held.borrow_mut().push(frame);
                    }
                }
            }
            if saturation && matches!(stage.get(), 2 | 3) {
                let elapsed = changed.get().elapsed();
                if !checked.get() && elapsed > Duration::from_millis(1500) {
                    if !player.has_pending_frame_work()
                        || player.frames().latest().is_some_and(|frame| {
                            frame.metadata().timestamp >= Duration::from_millis(700)
                        })
                    {
                        report("RESULT: FAIL saturated paused seek lost pending work");
                        return false;
                    }
                    report(&format!("SATURATION_PENDING retained={:?}", budget.usage()));
                    checked.set(true);
                }
                if !released.get() && elapsed > Duration::from_secs(2) {
                    held.borrow_mut().clear();
                    released.set(true);
                    report("SATURATION_RELEASED");
                }
            }
            match stage.get() {
                0 if (!saturation || held.borrow().len() == 3)
                    && state.position > Duration::from_millis(200)
                    && budget.counters().snapshot().gpu_imports > 0 =>
                {
                    let counts = budget.counters().snapshot();
                    if counts.preview != PixelTransferCount::default() {
                        report("RESULT: FAIL preview CPU transfer");
                        return false;
                    }
                    report(&format!("PRESENTED {:?}", counts));
                    if let Some(frame) = player.frames().latest() {
                        if geometry_smoke() {
                            report(&format!(
                                "VIDEO_GEOMETRY {:?} natural={:?}",
                                frame.metadata(),
                                state.natural_size
                            ));
                            #[cfg(target_os = "ios")]
                            if frame.metadata().rotation != FrameRotation::Clockwise90
                                || !frame.metadata().mirrored
                                || state.natural_size
                                    != Some(if crop_smoke() { (80, 144) } else { (90, 160) })
                            {
                                report("RESULT: FAIL native track orientation");
                                return false;
                            }
                        }
                        #[cfg(target_os = "ios")]
                        if crop_smoke() {
                            let r = frame.metadata().visible;
                            if (r.x, r.y, r.width, r.height) != (10, 6, 144, 80) {
                                report(&format!("RESULT: FAIL native crop {r:?}"));
                                return false;
                            }
                        }
                        report(&format!(
                            "PRESENTED_FRAME timestamp={:?} size={}x{}",
                            frame.metadata().timestamp,
                            frame.metadata().width,
                            frame.metadata().height
                        ));
                    }
                    if let Err(e) = player.pause() {
                        report(&format!("RESULT: FAIL pause {e}"));
                        return false;
                    }
                    stage.set(1);
                    changed.set(Instant::now());
                }
                1 if changed.get().elapsed() > Duration::from_millis(200)
                    && device_recovery_smoke()
                    && !recovery_started.replace(true) =>
                {
                    previous_generation.set(
                        player
                            .frames()
                            .latest()
                            .unwrap()
                            .metadata()
                            .identity
                            .generation,
                    );
                    previous_position.set(state.position);
                    stage.set(8);
                    if let Some(visual) = ElementComposition::get_element_visual(&view) {
                        visual.compositor().invoke_server_job_async(
                            |server| {
                                server.reset_all_gpu_resources();
                                server.render_interface().value();
                                Ok(())
                            },
                            false,
                        );
                    }
                }
                8 if player.frames().latest().is_some_and(|f| {
                    f.metadata().identity.generation > previous_generation.get()
                }) =>
                {
                    if player.requested_state() != ferroui_media_video::PlaybackIntent::Pause
                        || state.position.abs_diff(previous_position.get())
                            > Duration::from_millis(150)
                    {
                        report(&format!(
                            "RESULT: FAIL recovery position={:?} intent={:?}",
                            state.position,
                            player.requested_state()
                        ));
                        return false;
                    }
                    report(&format!(
                        "DEVICE_RECOVERY_PASS paused position={:?} generation={}",
                        state.position,
                        player
                            .frames()
                            .latest()
                            .unwrap()
                            .metadata()
                            .identity
                            .generation
                    ));
                    stage.set(1);
                    changed.set(Instant::now());
                }
                1 if changed.get().elapsed() > Duration::from_millis(200) => {
                    stage.set(2);
                    changed.set(Instant::now());
                    let stage = stage.clone();
                    let seek = player.seek(Duration::from_millis(750));
                    Dispatcher::ui_thread().spawn_local(async move {
                        match seek.await {
                            Ok(value) => {
                                report(&format!("SEEK_COMPLETED {value:?}"));
                                stage.set(3);
                            }
                            Err(e) => report(&format!("RESULT: FAIL seek {e}")),
                        }
                    });
                }
                3 if !saturation
                    || (released.get()
                        && player.frames().latest().is_some_and(|v| {
                            v.metadata().timestamp >= Duration::from_millis(700)
                        })) =>
                {
                    if saturation {
                        report("SATURATION_RECOVERED_PAUSED_FRAME");
                    }
                    stage.set(4);
                    let play = player.play();
                    Dispatcher::ui_thread().spawn_local(async move {
                        if let Err(e) = play.await {
                            report(&format!("RESULT: FAIL play {e}"));
                        }
                    });
                }
                4 if state.position > Duration::from_millis(1000) => {
                    stage.set(5);
                    let stop = player.stop();
                    Dispatcher::ui_thread().spawn_local(async move {
                        if let Err(e) = stop.await {
                            report(&format!("RESULT: FAIL stop {e}"));
                        }
                    });
                }
                5 if state.position == Duration::ZERO
                    && player
                        .frames()
                        .latest()
                        .is_some_and(|v| v.metadata().timestamp < Duration::from_millis(100)) =>
                {
                    report("STOP_FRAME_ZERO");
                    player.clear_source();
                    view.set_media_player(None, None);
                    stage.set(6);
                }
                _ => {}
            }
            true
        },
        Duration::from_millis(33),
        DispatcherPriority::NORMAL,
    );
}

async fn native_qr_smoke() {
    let fixture = include_bytes!("../../tests/media/fixtures/qr-multiple.pgm");
    let luma = &fixture[fixture.len() - 512 * 256..];
    #[cfg(target_os = "ios")]
    {
        use ferroui_media_qr::*;
        let frame = match ferroui_media_qr_apple::fixtures::luma_frame(luma, 512, 256) {
            Ok(v) => v,
            Err(e) => {
                report(&format!("NATIVE_QR FAIL fixture {e:?}"));
                return;
            }
        };
        let gate = QrAnalysisGate::new(QrOptions {
            max_age: Duration::from_secs(30),
            ..Default::default()
        });
        gate.activate(9, 1);
        let result =
            ferroui_media_qr_apple::recognize(gate.begin(frame, Duration::ZERO).unwrap()).await;
        match result {
            Ok(values)
                if values.len() == 2
                    && values
                        .iter()
                        .any(|v| v.text.as_deref() == Some("ferroui-native-qr-left"))
                    && values
                        .iter()
                        .any(|v| v.text.as_deref() == Some("ferroui-native-qr-right")) =>
            {
                report("NATIVE_QR PASS offline Vision multi-symbol retained CoreVideo frame")
            }
            other => report(&format!("NATIVE_QR FAIL {other:?}")),
        }
    }
    #[cfg(target_os = "android")]
    {
        use base64::Engine;
        use ferroui_android::interop::java::*;
        let bytes = base64::engine::general_purpose::STANDARD.encode(luma);
        let fixture = new_object(
            &JavaClass::find("org/ferroui/portablemedia/QrFixture"),
            "(Ljava/lang/String;)V",
            &[JavaValue::String(&bytes)],
        )
        .to_global();
        let start = Instant::now();
        ferroui_media_android::poll_native(move || {
            let text = ferroui_media_android::text(&fixture, "result");
            if !text.is_empty() {
                report(&format!("NATIVE_QR {text}"));
                return std::task::Poll::Ready(Ok::<(), ()>(()));
            }
            if start.elapsed() > Duration::from_secs(30) {
                report("NATIVE_QR FAIL timeout");
                return std::task::Poll::Ready(Ok::<(), ()>(()));
            }
            std::task::Poll::Pending
        })
        .await
        .ok();
    }
}

fn qr_action(
    actions: &Ref<ferroui_controls::StackPanel>,
    camera: &Ref<ferroui_media_camera_controls::CameraPreview>,
) {
    use ferroui_base::BoxedValue;
    use ferroui_controls::{automation::AutomationProperties, Button};
    let button = Button::new();
    button.set_content(Some(Rc::new("QR".to_string()) as BoxedValue));
    AutomationProperties::set_name(&button, Some("Toggle QR analysis"));
    let camera_errors = camera
        .errors()
        .subscribe(|e| report(&format!("CAMERA_ERROR {:?} {e}", e.kind)));
    let enabled = Rc::new(Cell::new(false));
    let state = enabled.clone();
    button.click(move |_, _| {
        state.set(!state.get());
        report(if state.get() {
            "QR_ENABLED"
        } else {
            "QR_DISABLED"
        });
    });
    actions.children().add(button);
    let weak = camera.downgrade();
    #[cfg(target_os = "android")]
    {
        let last_report = Cell::new(Instant::now());
        let analyzer = std::cell::RefCell::new(None::<ferroui_media_qr_android::AndroidQrAnalyzer>);
        DispatcherTimer::run(
            move || {
                let _keep_errors = &camera_errors;
                let Some(camera) = weak.upgrade() else {
                    analyzer.borrow_mut().take();
                    return false;
                };
                let session = camera.session().filter(|v| !v.is_stopped());
                if !enabled.get() || session.is_none() {
                    analyzer.borrow_mut().take();
                    return true;
                }
                if analyzer.borrow().is_none() {
                    match ferroui_media_qr_android::AndroidQrAnalyzer::attach(
                        session.as_ref().unwrap(),
                        Default::default(),
                    ) {
                        Ok(value) => {
                            analyzer.replace(Some(value));
                            report("QR_NATIVE_ATTACHED");
                        }
                        Err(e) => {
                            report(&format!("QR_ATTACH_ERROR {e:?}"));
                            enabled.set(false);
                            return true;
                        }
                    }
                }
                if last_report.get().elapsed() > Duration::from_secs(2) {
                    let frame = session.as_ref().unwrap().frames().latest();
                    report(&format!(
                        "QR_ANALYSIS {:?} preview={:?}",
                        analyzer.borrow().as_ref().unwrap().statistics(),
                        frame
                            .as_ref()
                            .map(|v| (v.metadata().identity.sequence, v.counters().snapshot()))
                    ));
                    last_report.set(Instant::now());
                }
                let result = analyzer.borrow().as_ref().unwrap().take();
                match result {
                    Ok(values) if !values.is_empty() => report(&format!("QR_SYMBOLS {values:?}")),
                    Err(e) => {
                        report(&format!("QR_ERROR {e:?}"));
                        enabled.set(false);
                        analyzer.borrow_mut().take();
                    }
                    _ => {}
                }
                true
            },
            Duration::from_millis(100),
            DispatcherPriority::NORMAL,
        );
    }
    #[cfg(target_os = "ios")]
    {
        use ferroui_media_qr::*;
        let gate = QrAnalysisGate::new(Default::default());
        let active = Cell::new(None);
        let started = Instant::now();
        DispatcherTimer::run(
            move || {
                let _keep_errors = &camera_errors;
                let Some(camera) = weak.upgrade() else {
                    gate.stop();
                    return false;
                };
                let session = camera.session().filter(|v| !v.is_stopped());
                if !enabled.get() || session.is_none() {
                    gate.stop();
                    active.set(None);
                    return true;
                }
                let Some(frame) = session.unwrap().frames().latest() else {
                    return true;
                };
                let id = frame.metadata().identity;
                if active.get() != Some((id.source, id.generation)) {
                    gate.activate(id.source, id.generation);
                    active.set(Some((id.source, id.generation)));
                }
                if let Ok(analysis) = gate.begin(frame, started.elapsed()) {
                    Dispatcher::ui_thread().spawn_local(async move {
                        match ferroui_media_qr_apple::recognize(analysis).await {
                            Ok(values) if !values.is_empty() => {
                                report(&format!("QR_SYMBOLS {values:?}"))
                            }
                            Err(QrError::DecodeFailure) => {
                                report("QR_ERROR native recognition failed")
                            }
                            _ => {}
                        }
                    });
                }
                true
            },
            Duration::from_millis(100),
            DispatcherPriority::NORMAL,
        );
    }
}

fn renderer_action(
    root: &Ref<Panel>,
    actions: &Ref<ferroui_controls::StackPanel>,
    camera: &Ref<ferroui_media_camera_controls::CameraPreview>,
    video: &Ref<MediaPlayerElement>,
    interop: &dyn ferroui_base::rendering::composition::ICompositionGpuInterop,
) {
    use ferroui_base::{
        layout::{HorizontalAlignment, VerticalAlignment},
        BoxedValue,
    };
    use ferroui_controls::{automation::AutomationProperties, Button};
    use ferroui_media::TextureRenderer;
    use ferroui_media_controls::RendererHost;
    use std::{cell::RefCell, sync::Arc};
    let source = ferroui_media::allocate_source_id();
    let budget = FrameBudget::new(Default::default());
    #[cfg(target_os = "ios")]
    let renderer = ferroui_media_metal::MetalTextureRenderer::from_interop(
        interop,
        budget.clone(),
        source,
        Arc::new(scene::Scene::default()),
    )
    .map(|v| Rc::new(v) as Rc<dyn TextureRenderer>);
    #[cfg(target_os = "android")]
    let renderer =
        ferroui_media_android::NativeGlFrameProducer::from_interop(interop, budget.clone()).map(
            |producer| {
                let scene = Arc::new(scene::Scene::new(producer.retirement_wakeup()));
                Rc::new(ferroui_media_opengl::GlTextureRenderer::new(
                    producer, source, scene,
                )) as Rc<dyn TextureRenderer>
            },
        );
    #[cfg(target_os = "android")]
    let probe_producer = Rc::new(RefCell::new(
        ferroui_media_android::NativeGlFrameProducer::from_interop(interop, budget.clone()).ok(),
    ));
    let renderer = match renderer {
        Ok(v) => v,
        Err(e) => {
            report(&format!("RENDERER_SETUP_ERROR {e}"));
            return;
        }
    };
    let host = RendererHost::new();
    host.set_width(220.);
    host.set_height(220.);
    host.set_horizontal_alignment(HorizontalAlignment::Right);
    host.set_vertical_alignment(VerticalAlignment::Bottom);
    host.set_is_visible(false);
    let requested = Rc::new(Cell::new(renderer_smoke()));
    #[cfg(target_os = "ios")]
    let importer = Rc::new(ferroui_media_video_apple::AppleTextureImporter)
        as Rc<dyn ferroui_media_controls::TextureFrameImporter>;
    #[cfg(target_os = "android")]
    let importer = Rc::new(ferroui_media_android::NativeGlTextureImporter)
        as Rc<dyn ferroui_media_controls::TextureFrameImporter>;
    host.set_renderer(Some(renderer), source, Some(importer));
    let recovery_budget = budget.clone();
    #[cfg(target_os = "android")]
    let recovery_probe = probe_producer.clone();
    host.set_renderer_factory(Some(Rc::new(move |interop, source| {
        #[cfg(target_os = "ios")]
        {
            Ok((
                Rc::new(ferroui_media_metal::MetalTextureRenderer::from_interop(
                    interop,
                    recovery_budget.clone(),
                    source,
                    Arc::new(scene::Scene::default()),
                )?),
                Rc::new(ferroui_media_video_apple::AppleTextureImporter),
            ))
        }
        #[cfg(target_os = "android")]
        {
            let producer = NativeGlFrameProducer::from_interop(interop, recovery_budget.clone())?;
            recovery_probe.replace(Some(producer.clone()));
            let scene = Arc::new(scene::Scene::new(producer.retirement_wakeup()));
            Ok((
                Rc::new(ferroui_media_opengl::GlTextureRenderer::new(
                    producer, source, scene,
                )),
                Rc::new(NativeGlTextureImporter),
            ))
        }
    })));

    // Retain the diagnostic subscription for this view's lifetime.
    let errors = host
        .errors()
        .subscribe(|e| report(&format!("RENDERER_ERROR {e}")));
    let weak = host.downgrade();
    let camera = camera.downgrade();
    let video = video.downgrade();
    let current = RefCell::new(None::<Arc<FrameMailbox>>);
    let last_message = RefCell::new(String::new());
    let probed = Cell::new(None::<u64>);
    let retiring = Cell::new(false);
    let wanted = requested.clone();
    let last_report = Cell::new(Instant::now());
    DispatcherTimer::run(
        move || {
            let _keep_errors = &errors;
            let Some(host) = weak.upgrade() else {
                return false;
            };
            if !wanted.get() {
                if host.is_visible() {
                    host.set_is_visible(false);
                    retiring.set(true);
                }
                if current.borrow().is_some() {
                    retiring.set(true);
                    host.set_inputs(Vec::new());
                    current.replace(None);
                }
                if retiring.get() && budget.usage().frames == 0 {
                    report("RENDERER_RETIRED frames=0");
                    retiring.set(false);
                }
                return true;
            }
            let input = camera
                .upgrade()
                .filter(|v| v.is_visible())
                .and_then(|v| v.session())
                .filter(|s| !s.is_stopped())
                .map(|s| s.frames())
                .or_else(|| {
                    video
                        .upgrade()
                        .filter(|v| v.is_visible())
                        .and_then(|v| v.media_player())
                        .map(|p| p.frames())
                });
            let changed = match (current.borrow().as_ref(), input.as_ref()) {
                (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
                (None, None) => false,
                _ => true,
            };
            if changed {
                host.set_inputs(input.iter().cloned().collect());
                current.replace(input);
            }
            if current.borrow().is_none() {
                if host.is_visible() {
                    host.set_is_visible(false);
                    retiring.set(true);
                }
                if retiring.get() && budget.usage().frames == 0 {
                    report("RENDERER_RETIRED frames=0");
                    retiring.set(false);
                }
                return true;
            }
            retiring.set(false);
            host.set_is_visible(true);
            let input = current.borrow().as_ref().and_then(|s| s.latest());
            let output = host.output().and_then(|s| s.latest());
            if is_smoke() && input.is_some() {
                if let Some(frame) = output
                    .clone()
                    .filter(|f| probed.get() != Some(f.metadata().identity.source))
                {
                    probed.set(Some(frame.metadata().identity.source));
                    #[cfg(target_os = "ios")]
                    match scene::probe(&frame, geometry_smoke()) {
                        Ok(()) => report("RENDERER_PIXELS_PASS transparent projection coverage"),
                        Err(e) => report(&format!("RENDERER_PIXELS_FAIL {e}")),
                    }
                    #[cfg(target_os = "android")]
                    if let Some(producer) = probe_producer.borrow().clone() {
                        Dispatcher::ui_thread().spawn_local(async move {
                            match scene::probe(frame, producer, geometry_smoke()).await {
                                Ok(()) => {
                                    report("RENDERER_PIXELS_PASS transparent projection coverage")
                                }
                                Err(e) => report(&format!("RENDERER_PIXELS_FAIL {e}")),
                            }
                        });
                    }
                }
            }
            if last_report.get().elapsed() > Duration::from_secs(2) {
                let message = format!(
                    "RENDERER_SHARED_INPUT {:?} output={:?} budget={:?} counters={:?}",
                    input.as_ref().map(|v| v.metadata().identity),
                    output.as_ref().map(|v| v.metadata().identity),
                    budget.usage(),
                    budget.counters().snapshot()
                );
                if *last_message.borrow() != message {
                    report(&message);
                    last_message.replace(message);
                }

                last_report.set(Instant::now());
            }
            true
        },
        Duration::from_millis(33),
        DispatcherPriority::NORMAL,
    );
    let button = Button::new();
    button.set_content(Some(Rc::new("3D".to_string()) as BoxedValue));
    AutomationProperties::set_name(&button, Some("Toggle shared texture renderer"));
    let weak = host.downgrade();
    button.click(move |_, _| {
        if let Some(host) = weak.upgrade() {
            requested.set(!requested.get());
            if !requested.get() {
                host.set_is_visible(false);
            }
            report(if requested.get() {
                "RENDERER_ENABLED"
            } else {
                "RENDERER_DISABLED"
            });
        }
    });
    actions.children().add(button);
    root.children().add(host);
}
