use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate,
    rendering::composition::{frames::*, ElementComposition},
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
    FerroObjectImpl, Ref,
};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Control, NewApplication, Panel,
};
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
            .add(ferroui_themes_simple::SimpleTheme::new().as_style());
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
pub struct Delegate;
#[cfg(target_os = "ios")]
impl ferroui_ios::FerroApplicationDelegate for Delegate {
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
fn is_smoke() -> bool {
    #[cfg(target_os = "ios")]
    return std::env::args().any(|v| v == "--smoke");
    #[cfg(target_os = "android")]
    {
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
                    &[JavaValue::String("mediaSmoke"), JavaValue::Boolean(false)],
                )
            })
    }
}
fn report(value: &str) {
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
            view.set_autoplay(smoke);
            let weak = view.downgrade();
            let weak_root = weak_root.clone();
            let phase = phase.clone();
            phase.set(1);
            Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                let Some(interop) = compositor.try_get_composition_gpu_interop().await else {
                    phase.set(0);
                    return;
                };
                phase.set(2);
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
                    encoded: include_bytes!("../../tests/media/fixtures/avc-aac.mp4")
                        .as_slice()
                        .into(),
                };
                match view.set_source(source, provider, importer).await {
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
    let host=ferroui_controls::UserControl::new();
    host.set_content(Some(Control::boxed(root)));
    ferroui_controls::TopLevel::set_auto_safe_area_padding(&host,true);
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
            }) as Rc<dyn CameraProvider>,
            Rc::new(ferroui_media_android::NativeGlTextureImporter)
                as Rc<dyn ferroui_media_controls::TextureFrameImporter>,
        )
    };
    let capture = CameraCapture::new(provider, budget.clone());
    let devices = capture.devices();
    Dispatcher::ui_thread().invoke_async_task_local(move || async move {
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
    let video = video.downgrade();
    start.click(move |_, _| {
        if let Some(video) = video.upgrade() {
            if let Some(player) = video.media_player() {
                player.clear_source();
            }
            video.set_is_visible(false);
        }
        let Some(camera) = weak.upgrade() else {
            return;
        };
        camera.set_is_visible(true);
        let pending = camera.start(capture.clone(), Default::default(), importer.clone());
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            match pending.await {
                Ok(()) => report("CAMERA_STARTED"),
                Err(error) => report(&format!("CAMERA_START_ERROR {:?} {error}", error.kind)),
            }
        });
    });
    let weak = camera.downgrade();
    stop.click(move |_, _| {
        if let Some(camera) = weak.upgrade() {
            camera.stop();
            report("CAMERA_STOPPED");
        }
    });
    let weak = camera.downgrade();
    photo.click(move |_, _| {
        let Some(session) = weak.upgrade().and_then(|v| v.session()) else {
            return;
        };
        let pending = session.photo();
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
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
                Err(error) => report(&format!("CAMERA_PHOTO_ERROR {error}")),
            }
        });
    });
    let root_weak = root.downgrade();
    let weak = camera.downgrade();
    navigate.click(move |_, _| {
        if let (Some(root), Some(camera)) = (root_weak.upgrade(), weak.upgrade()) {
            root.children().remove(&camera);
            report("CAMERA_PREVIEW_DETACHED");
        }
    });
    actions.children().add(start);
    actions.children().add(stop);
    actions.children().add(photo);
    actions.children().add(navigate);
    root.children().add(actions);
}
fn start_smoke(view: &Ref<MediaPlayerElement>, budget: FrameBudget) {
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
                    "RESULT: FAIL timeout stage={} retained={:?}",
                    stage.get(),
                    budget.usage()
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
                report("RESULT: PASS playback pause seek resume stop frame-zero detach retained=0");
                return false;
            }
            let Some(player) = view.media_player() else {
                return true;
            };
            let state = player.playback_session();
            if state.state == MediaPlaybackState::Failed {
                report(&format!("RESULT: FAIL {:?}", state.error));
                return false;
            }
            match stage.get() {
                0 if state.position > Duration::from_millis(200)
                    && budget.counters().snapshot().gpu_imports > 0 =>
                {
                    let counts = budget.counters().snapshot();
                    if counts.preview != PixelTransferCount::default() {
                        report("RESULT: FAIL preview CPU transfer");
                        return false;
                    }
                    report(&format!("PRESENTED {:?}", counts));
                    if let Some(frame) = player.frames().latest() {
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
                1 if changed.get().elapsed() > Duration::from_millis(200) => {
                    stage.set(2);
                    let stage = stage.clone();
                    let seek = player.seek(Duration::from_millis(750));
                    Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                        match seek.await {
                            Ok(value) => {
                                report(&format!("SEEK_COMPLETED {value:?}"));
                                stage.set(3);
                            }
                            Err(e) => report(&format!("RESULT: FAIL seek {e}")),
                        }
                    });
                }
                3 => {
                    stage.set(4);
                    let play = player.play();
                    Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                        if let Err(e) = play.await {
                            report(&format!("RESULT: FAIL play {e}"));
                        }
                    });
                }
                4 if state.position > Duration::from_millis(1000) => {
                    stage.set(5);
                    let stop = player.stop();
                    Dispatcher::ui_thread().invoke_async_task_local(move || async move {
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
