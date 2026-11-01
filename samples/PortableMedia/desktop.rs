use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate,
    media::Stretch,
    rendering::composition::{frames::*, ElementComposition},
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
    FerroObjectImpl, Ref,
};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Control, NewApplication, Panel,
    Window,
};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_media_video::{MediaPlaybackState, MediaSource};

use ferroui_media_video_controls::MediaPlayerElement;
use std::{
    cell::Cell,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
static FAILED: AtomicBool = AtomicBool::new(false);
#[path = "camera.rs"]
mod camera;
#[path = "providers.rs"]
mod providers;
#[cfg(target_os = "macos")]
#[path = "renderer.rs"]
mod renderer;
#[cfg(not(target_os = "macos"))]
#[path = "native_renderer.rs"]
mod renderer;
#[repr(C)]
struct App {
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
        this.styles()
            .add(ferroui_themes_fluent::FluentTheme::new().as_style());
    }
    fn on_framework_initialization_completed(this: &Self) {
        if let Some(lifetime) = this
            .application_lifetime()
            .as_ref()
            .and_then(|l| l.as_classic_desktop_style_application_lifetime())
        {
            if std::env::args()
                .nth(1)
                .as_deref()
                .is_some_and(|a| a == "--camera" || a == "--camera-discovery")
            {
                let window = Window::new();
                window.set_title(Some("FerroUI camera".into()));
                window.set_width(800.);
                window.set_height(500.);
                window.set_content(Some(Control::boxed(camera::create(&window))));
                lifetime.set_main_window(Some(window));
                Self::parent_on_framework_initialization_completed(this);
                return;
            }
            let view = MediaPlayerElement::new();
            view.set_autoplay(true);
            view.set_are_transport_controls_enabled(true);
            view.set_stretch(Stretch::Uniform);
            let window = Window::new();
            window.set_title(Some("FerroUI shared-texture media".into()));
            window.set_width(800.);
            window.set_height(500.);
            let root = Panel::new();
            root.children().add(&view);
            window.set_content(Some(Control::boxed(root.clone())));
            let weak_root = root.downgrade();
            let weak = view.downgrade();
            let weak_window = window.downgrade();
            let attempts = Cell::new(0);
            DispatcherTimer::run(
                move || {
                    let Some(view) = weak.upgrade() else {
                        return false;
                    };
                    attempts.set(attempts.get() + 1);
                    let Some(visual) = ElementComposition::get_element_visual(&view) else {
                        if attempts.get() > 100 {
                            FAILED.store(true, Ordering::Relaxed);
                            if let Some(window) = weak_window.upgrade() {
                                window.close();
                            }
                            return false;
                        }
                        return true;
                    };
                    let compositor = visual.compositor().clone();
                    let weak = view.downgrade();
                    let weak_window = weak_window.clone();
                    let weak_root = weak_root.clone();
                    Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                        let Some(interop) = compositor.try_get_composition_gpu_interop().await
                        else {
                            FAILED.store(true, Ordering::Relaxed);
                            if let Some(window) = weak_window.upgrade() {
                                window.close();
                            }
                            return;
                        };
                        let Some(view) = weak.upgrade() else {
                            return;
                        };
                        let budget = FrameBudget::new(FrameBudgetLimits::default());
                        let provider = match providers::video(&*interop, budget.clone()) {
                            Ok(v) => v,
                            Err(e) => {
                                eprintln!("PROVIDER_FAILED {e}");
                                FAILED.store(true, Ordering::Relaxed);
                                if let Some(window) = weak_window.upgrade() {
                                    window.close();
                                }
                                return;
                            }
                        };
                        let path = std::env::args().nth(1).expect("media source argument");
                        let uri = path.clone();
                        let source = if std::env::args().any(|a| a == "--resource") {
                            let encoded = match std::fs::read(&path) {
                                Ok(encoded) => encoded,
                                Err(error) => {
                                    eprintln!("RESOURCE_READ_FAILED {error}");
                                    FAILED.store(true, Ordering::Relaxed);
                                    if let Some(window) = weak_window.upgrade() {
                                        window.close();
                                    }
                                    return;
                                }
                            };
                            MediaSource::Resource {
                                name: "portable-media-fixture.mp4".into(),
                                content_type: "video/mp4".into(),
                                encoded: encoded.into(),
                            }
                        } else if path.starts_with("https://") || path.starts_with("http://") {
                            MediaSource::Url(path)
                        } else {
                            MediaSource::File(path.into())
                        };
                        let recovery_budget = budget.clone();
                        view.set_provider_factory(Some(Rc::new(move |interop| {
                            Ok((
                                providers::video(interop, recovery_budget.clone())?,
                                providers::importer(),
                            ))
                        })));
                        #[cfg(target_os = "macos")]
                        let pending = if std::env::args().any(|a| a == "--source-uri") {
                            view.set_source_uri(Some(uri));
                            wait_for_declared_source(&view)
                        } else {
                            view.set_source(source, provider, providers::importer())
                        };
                        #[cfg(not(target_os = "macos"))]
                        let pending = {
                            let _ = uri;
                            view.set_source(source, provider, providers::importer())
                        };
                        if let Err(error) = pending.await {
                            eprintln!("OPEN_FAILED {error}");
                            FAILED.store(true, Ordering::Relaxed);
                            if let Some(window) = weak_window.upgrade() {
                                window.close();
                            }
                            return;
                        }
                        println!(
                            "OPENED native_platform_provider shared_texture=true decode=unknown"
                        );
                        if let (Some(root), Some(player)) =
                            (weak_root.upgrade(), view.media_player())
                        {
                            renderer::install(&root, player.frames(), &*interop);
                        }

                        if std::env::args().any(|a| a == "--smoke") {
                            if let Some(window) = weak_window.upgrade() {
                                smoke(&window, &view, budget);
                            }
                        }
                    });
                    false
                },
                Duration::from_millis(50),
                DispatcherPriority::NORMAL,
            );
            lifetime.set_main_window(Some(window));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}
#[cfg(target_os = "macos")]
fn wait_for_declared_source(
    view: &Ref<MediaPlayerElement>,
) -> ferroui_media_video::MediaFuture<()> {
    let weak = view.downgrade();
    let started = Instant::now();
    ferroui_media_apple::poll_native(move || {
        use std::task::Poll;
        let Some(view) = weak.upgrade() else {
            return Poll::Ready(Err(ferroui_media::MediaError::cancelled()));
        };
        if let Some(player) = view.media_player() {
            let state = player.playback_session();
            if state.state == MediaPlaybackState::Failed {
                return Poll::Ready(Err(state.error.unwrap()));
            }
            if !matches!(
                state.state,
                MediaPlaybackState::Closed | MediaPlaybackState::Opening
            ) {
                return Poll::Ready(Ok(()));
            }
        }
        if started.elapsed() > Duration::from_secs(15) {
            return Poll::Ready(Err(ferroui_media::MediaError::new(
                ferroui_media::MediaErrorKind::NativeFailure,
                "Source binding did not prepare",
            )));
        }
        Poll::Pending
    })
}
fn smoke(window: &Ref<Window>, view: &Ref<MediaPlayerElement>, budget: FrameBudget) {
    let view = view.downgrade();
    let window = window.downgrade();
    let started = Instant::now();
    let stage = Rc::new(Cell::new(0u8));
    let changed = Cell::new(Instant::now());
    let imports_before_stop = Cell::new(0u64);
    let recovery_started = Cell::new(false);
    let previous_generation = Cell::new(0u64);
    let previous_position = Cell::new(Duration::ZERO);
    let restart_started = Cell::new(false);
    let sequence_before_restart = Cell::new(0u64);
    DispatcherTimer::run(
        move || {
            let (Some(view), Some(window)) = (view.upgrade(), window.upgrade()) else {
                return false;
            };
            if started.elapsed() > Duration::from_secs(15) {
                eprintln!(
                    "SMOKE_TIMEOUT stage={} retained={:?}",
                    stage.get(),
                    budget.usage()
                );
                FAILED.store(true, Ordering::Relaxed);
                window.close();
                return false;
            }
            if stage.get() == 7 && budget.usage().frames == 0 && !renderer::retained() {
                #[cfg(target_os = "macos")]
                if !renderer::probes_complete() {
                    return true;
                }
                println!("NATIVE_MEDIA_SMOKE_PASS retained_frames=0");
                window.close();
                return false;
            }
            let Some(player) = view.media_player() else {
                return true;
            };
            let state = player.playback_session();
            if state.state == MediaPlaybackState::Failed && stage.get() != 8 {
                eprintln!("NATIVE_FAILED {:?}", state.error);
                FAILED.store(true, Ordering::Relaxed);
                window.close();
                return false;
            }
            if std::env::args().any(|a| a == "--ended-restart-smoke") && !restart_started.get() {
                if state.state == MediaPlaybackState::Ended {
                    sequence_before_restart.set(
                        player
                            .frames()
                            .latest()
                            .map_or(0, |f| f.metadata().identity.sequence),
                    );
                    restart_started.set(true);
                    stage.set(9);
                    let pending = player.play();
                    let stage = stage.clone();
                    Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                        match pending.await {
                            Ok(()) => {
                                println!("ENDED_RESTART_COMPLETED");
                                stage.set(0);
                            }
                            Err(error) => {
                                eprintln!("ENDED_RESTART_FAILED {error}");
                                FAILED.store(true, Ordering::Relaxed);
                            }
                        }
                    });
                }
                return true;
            }
            match stage.get() {
                0 if player.frames().latest().is_some()
                    && state.position > Duration::from_millis(200)
                    && (!restart_started.get()
                        || (state.state == MediaPlaybackState::Playing
                            && state.position < Duration::from_millis(1500)
                            && player.frames().latest().is_some_and(|f| {
                                f.metadata().identity.sequence > sequence_before_restart.get()
                                    && f.metadata().timestamp < Duration::from_secs(1)
                            }))) =>
                {
                    let frame = player.frames().latest().unwrap();
                    if std::env::args().any(|v| v == "--geometry-smoke") {
                        println!(
                            "VIDEO_GEOMETRY {:?} natural={:?}",
                            frame.metadata(),
                            state.natural_size
                        );
                        #[cfg(target_os = "macos")]
                        if frame.metadata().rotation != FrameRotation::Clockwise90
                            || !frame.metadata().mirrored
                            || state.natural_size
                                != Some(if std::env::args().any(|v| v == "--crop-smoke") {
                                    (80, 144)
                                } else {
                                    (90, 160)
                                })
                        {
                            eprintln!("NATIVE_GEOMETRY_FAILED");
                            FAILED.store(true, Ordering::Relaxed);
                            window.close();
                            return false;
                        }
                    }
                    #[cfg(target_os = "macos")]
                    if std::env::args().any(|v| v == "--crop-smoke") {
                        let r = frame.metadata().visible;
                        if (r.x, r.y, r.width, r.height) != (10, 6, 144, 80) {
                            eprintln!("NATIVE_CROP_FAILED {r:?}");
                            FAILED.store(true, Ordering::Relaxed);
                            window.close();
                            return false;
                        }
                    }
                    let counts = frame.counters().snapshot();
                    if counts.gpu_imports == 0 {
                        return true;
                    }
                    println!(
                        "PRESENTED position={:?} imports={} gpu_copies={} gpu_conversions={} color={:?} preview={:?}",
                        state.position, counts.gpu_imports, counts.gpu_copies, counts.gpu_conversions, frame.metadata().color, counts.preview
                    );
                    assert_eq!(counts.preview, PixelTransferCount::default());
                    assert_eq!(counts.gpu_copies, 0);
                    player.pause().unwrap();
                    stage.set(1);
                    changed.set(Instant::now());
                }
                1 if changed.get().elapsed() > Duration::from_millis(150)
                    && std::env::args().any(|a| a == "--device-recovery-smoke")
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
                        eprintln!(
                            "DEVICE_RECOVERY_FAILED position={:?} intent={:?}",
                            state.position,
                            player.requested_state()
                        );
                        FAILED.store(true, Ordering::Relaxed);
                        window.close();
                        return false;
                    }
                    println!(
                        "DEVICE_RECOVERY_PASS paused position={:?} generation={}",
                        state.position,
                        player
                            .frames()
                            .latest()
                            .unwrap()
                            .metadata()
                            .identity
                            .generation
                    );
                    stage.set(1);
                    changed.set(Instant::now());
                }
                1 if changed.get().elapsed() > Duration::from_millis(150) => {
                    stage.set(2);
                    let seek = player.seek(Duration::from_millis(500));
                    let stage = stage.clone();
                    Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                        match seek.await {
                            Ok(achieved) => {
                                println!("SEEK_COMPLETED {achieved:?}");
                                stage.set(3);
                            }
                            Err(e) => {
                                eprintln!("SEEK_FAILED {e}");
                                FAILED.store(true, Ordering::Relaxed);
                            }
                        }
                    });
                }
                3 => {
                    stage.set(4);
                    let play = view.play();
                    Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                        if play.await.is_err() {
                            FAILED.store(true, Ordering::Relaxed);
                        }
                    });
                    changed.set(Instant::now());
                }
                4 if changed.get().elapsed() > Duration::from_millis(200) => {
                    imports_before_stop.set(budget.counters().snapshot().gpu_imports);
                    stage.set(5);
                    let stopped = player.stop();
                    let stage = stage.clone();
                    Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                        match stopped.await {
                            Ok(achieved) => {
                                println!("STOP_COMPLETED {achieved:?}");
                                assert!(achieved <= Duration::from_millis(40));
                                stage.set(6);
                            }
                            Err(error) => {
                                eprintln!("STOP_FAILED {error}");
                                FAILED.store(true, Ordering::Relaxed);
                            }
                        }
                    });
                }
                6 if player.frames().latest().is_some_and(|frame| {
                    frame.metadata().timestamp <= Duration::from_millis(40)
                }) && budget.counters().snapshot().gpu_imports > imports_before_stop.get() =>
                {
                    println!("STOP_FRAME_ZERO_IMPORTED");
                    player.clear_source();
                    view.set_media_player(None, None);
                    stage.set(7);
                }
                7 if budget.usage().frames == 0 && !renderer::retained() => {
                    println!("NATIVE_MEDIA_SMOKE_PASS retained_frames=0");
                    window.close();
                    return false;
                }
                _ => {}
            }
            true
        },
        Duration::from_millis(33),
        DispatcherPriority::NORMAL,
    );
}
pub fn run() -> std::process::ExitCode {
    #[cfg(not(all(windows, feature = "synthetic-camera")))]
    if std::env::args().any(|a| a == "--synthetic-camera" || a == "--camera-smoke") {
        eprintln!("Synthetic camera requires a Windows build with --features synthetic-camera");
        return std::process::ExitCode::FAILURE;
    }
    if std::env::args().nth(1).is_none() {
        eprintln!("Usage: portable-media <file-or-url> [--smoke] [--resource] | --camera | --camera-discovery");
        return std::process::ExitCode::FAILURE;
    }
    let args: Vec<_> = std::env::args().skip(1).collect();
    let builder = AppBuilder::configure::<App>().use_platform_detect();
    #[cfg(target_os = "linux")]
    let builder = builder.with(Rc::new(ferroui_x11::X11PlatformOptions {
        rendering_mode: vec![ferroui_x11::X11RenderingMode::Egl],
        gl_profiles: vec![ferroui_opengl::GlVersion::new(
            ferroui_opengl::GlProfileType::OpenGLES,
            3,
            0,
        )],
        ..Default::default()
    }));
    let result = builder.start_with_classic_desktop_lifetime(&args);
    if FAILED.load(Ordering::Relaxed) {
        std::process::ExitCode::FAILURE
    } else {
        std::process::ExitCode::from(result as u8)
    }
}
