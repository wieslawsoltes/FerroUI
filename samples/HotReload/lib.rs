//! Durable application types live in this library; patchable function bodies live in program.rs.
#![cfg(debug_assertions)]
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};
use ferroui_controls::{
    Application, ApplicationImpl, Button, ContentControl, NewApplication, Window,
};
use ferroui_hot_reload::{ComponentReloadTarget, ReloadManifest, ReloadTarget};
#[cfg(feature = "patch-control")]
mod development;
mod refresh;
mod view_builder;
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
ferroui_base::ferro_thread_local! {
    static COUNT: Cell<i32> = const { Cell::new(0) };
    static ACTION: RefCell<Option<Rc<dyn Fn() -> i32>>> = RefCell::new(None);
    static VIEW_BUILDER: RefCell<Option<Rc<dyn Fn(String) -> String>>> = RefCell::new(None);
    static RESTART_PARTICIPANTS: RefCell<Vec<Rc<dyn ferroui_hot_reload::RestartParticipant>>> = RefCell::new(Vec::new());
}
pub fn increment(amount: i32) -> i32 {
    COUNT.with(|count| {
        count.set(count.get() + amount);
        count.get()
    })
}
pub fn set_action(action: Rc<dyn Fn() -> i32>) {
    ACTION.with(|slot| *slot.borrow_mut() = Some(action));
}
/// Install the executable's patch-aware, pure candidate markup builder.
pub fn set_view_builder(builder: Rc<dyn Fn(String) -> String>) {
    VIEW_BUILDER.with(|slot| *slot.borrow_mut() = Some(builder));
}
/// Register stable-library application work which must quiesce before restart
/// state capture. Its resume guard is released on cancelled/failed handoff.
pub fn register_restart_participant(participant: Rc<dyn ferroui_hot_reload::RestartParticipant>) {
    RESTART_PARTICIPANTS.with(|participants| participants.borrow_mut().push(participant));
}
#[cfg(feature = "patch-control")]
fn begin_restart_handoff(
    window: &Window,
) -> Result<ferroui_hot_reload::RestartHandoff, ferroui_hot_reload::ReloadError> {
    let participants = RESTART_PARTICIPANTS.with(|participants| participants.borrow().clone());
    ferroui_hot_reload::RestartHandoff::begin(window, participants)
}
#[repr(C)]
pub struct App {
    base: Application,
    timer: RefCell<Option<Rc<DispatcherTimer>>>,
    startup_subscription: RefCell<Option<Rc<dyn ferroui_base::reactive::IDisposable>>>,
}
ferro_class!(App: Application);
ferro_class_info!(App { new: App::new });
ferro_impl_classes!(App: FerroObjectImpl);
impl App {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
            timer: RefCell::new(None),
            startup_subscription: RefCell::new(None),
        })
    }
}
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        Self::new()
    }
}
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        ferroui_controls::register_types();
        ferroui_markup_xaml::register_types();
        this.styles()
            .add(ferroui_themes_simple::SimpleTheme::new().as_style());
    }
    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        let Some(lifetime) = lifetime
            .as_ref()
            .and_then(|lifetime| lifetime.as_classic_desktop_style_application_lifetime())
        else {
            return;
        };
        let startup =
            ferroui_hot_patch::restart::RestartStartup::from_environment("hot-reload-demo", 1)
                .expect("read compatible restart state");
        let window = Window::new();
        window.set_width(640.0);
        window.set_height(400.0);
        if let Some(startup) = &startup {
            assert_eq!(
                startup.state.navigation.as_deref(),
                Some("main"),
                "unknown navigation identity"
            );
            let counter = startup.state.application_state["counter"]
                .as_i64()
                .and_then(|value| i32::try_from(value).ok())
                .expect("valid saved counter");
            COUNT.with(|count| count.set(counter));
            if let Some(placement) = startup
                .state
                .windows
                .iter()
                .find(|window| window.key == "main")
            {
                window.set_width(placement.size[0]);
                window.set_height(placement.size[1]);
                window.set_position(ferroui_base::PixelPoint::new(
                    placement.position[0],
                    placement.position[1],
                ));
                if placement.maximized {
                    window.set_window_state(ferroui_controls::WindowState::Maximized);
                }
            }
        }
        window.set_title(Some(format!(
            "Hot reload — counter {}",
            COUNT.with(Cell::get)
        )));
        let host = ContentControl::new();
        window.set_content(Some(Rc::new(host.clone())));
        let references = Rc::new(refresh::CounterReferences::default());
        let event_references = references.clone();
        let weak_window = window.downgrade();
        host.add_handler(Button::click_event(), move |_, args| {
            let count = ACTION.with(|action| action.borrow().as_ref().map(|action| action()));
            #[cfg(feature = "patch-control")]
            if std::env::var_os("FERROUI_TRACE_CLICKS").is_some() {
                eprintln!("FERROUI_COUNTER_EVENT={}", serde_json::json!({
                    "counter": count,
                    "source_is_button": args.source().is_some_and(|source| source.cast::<Button>().is_some()),
                    "unix_ns": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos().to_string(),
                }));
            }
            if let (Some(window), Some(count)) = (weak_window.upgrade(), count) {
                window.set_title(Some(format!("Hot reload — counter {count}")));
                event_references.update(count);
            }
        });
        let mut manifest = ReloadManifest::parse(include_str!(env!("FERROUI_HOT_RELOAD_MANIFEST")))
            .expect("embedded development manifest");
        let uri = manifest
            .files
            .keys()
            .find(|uri| uri.ends_with("/View.xaml"))
            .expect("View.xaml manifest entry")
            .clone();
        if let Some(override_path) = std::env::var_os("FERROUI_HOT_VIEW") {
            let override_path = std::path::PathBuf::from(override_path);
            let old_root = manifest.files[&uri].parent().unwrap().to_owned();
            let new_root = override_path.parent().expect("override source directory");
            for path in manifest.files.values_mut() {
                let relative = path
                    .strip_prefix(&old_root)
                    .expect("sample source belongs to its document root");
                *path = new_root.join(relative);
            }
            manifest.files.insert(uri.clone(), override_path);
        }
        let source = manifest.sources[&uri].clone();
        let watcher = manifest.watcher(Duration::from_millis(40));
        let mut session = manifest.session().expect("register manifest baseline");
        let target: Rc<dyn ReloadTarget> = Rc::new(view_builder::BuiltView {
            component: ComponentReloadTarget::content_control(
                &host,
                uri.clone(),
                source.clone(),
                RuntimeXamlLoaderConfiguration::default(),
            )
            .with_refresh(references),
            uri: uri.clone(),
            documents: manifest.files.keys().cloned().collect(),
        });
        session.register_target(&target);
        session.submit(uri, source);
        let initial = session.apply_pending().expect("initial markup revision");
        eprintln!("{initial:?}");
        if initial.status == ferroui_hot_reload::ReloadStatus::Applied {
            if let Some(startup) = startup {
                let startup = RefCell::new(Some(startup));
                *this.startup_subscription.borrow_mut() = Some(window.opened(move || {
                    if let Some(startup) = startup.borrow_mut().take() {
                        startup
                            .acknowledge()
                            .expect("acknowledge initialized replacement");
                    }
                }));
            }
        }
        let session = RefCell::new(session);
        #[cfg(feature = "patch-control")]
        // SAFETY: patch-control is an explicit internal supervisor mode. Its stdin
        // parent must own the pinned compiler and isolated admitted build inputs.
        let development = RefCell::new(unsafe { development::Development::connect() });
        let weak_host = host.downgrade();
        let smoke_exit = std::env::var("FERROUI_SMOKE_EXIT_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok());
        let started = std::time::Instant::now();
        let smoke_window = window.downgrade();
        let timer = DispatcherTimer::with_callback(
            Duration::from_millis(40),
            DispatcherPriority::BACKGROUND,
            move |timer| {
                if smoke_exit.is_some_and(|milliseconds| {
                    started.elapsed() >= Duration::from_millis(milliseconds)
                }) {
                    timer.stop();
                    if let Some(window) = smoke_window.upgrade() {
                        window.close();
                    }
                    return;
                }
                let _keep_target = &target;
                if weak_host.upgrade().is_none() {
                    timer.stop();
                    return;
                }
                let mut session = session.borrow_mut();
                if let Some(batch) = watcher.changes.try_iter().last() {
                    session.submit_files(batch);
                }
                #[cfg(feature = "patch-control")]
                if let Some(development) = development.borrow_mut().as_mut() {
                    if let (Some(host), Some(window)) =
                        (weak_host.upgrade(), smoke_window.upgrade())
                    {
                        development.pump(&host, &window, &mut session);
                    }
                    if development.building() {
                        return;
                    }
                }
                if let Some(outcome) = session.apply_pending() {
                    if let Some(window) = smoke_window.upgrade() {
                        ferroui_hot_reload::record_frame_timing(&window, &outcome);
                    }
                    eprintln!("{outcome:?}");
                }
            },
        );
        *this.timer.borrow_mut() = Some(timer);
        lifetime.set_main_window(Some(window));
    }
}

/// The development supervisor invokes this on the UI thread after the replacement build succeeds.
pub fn capture_restart_state(window: &Window) -> ferroui_hot_patch::restart::RestartState {
    use ferroui_hot_patch::restart::{RestartState, WindowPlacement};
    let mut state = RestartState::new(
        "hot-reload-demo".into(),
        1,
        serde_json::json!({ "counter": COUNT.with(Cell::get) }),
    );
    state.navigation = Some("main".into());
    let position = window.position();
    state.windows.push(WindowPlacement {
        key: "main".into(),
        position: [position.x, position.y],
        size: [window.width(), window.height()],
        maximized: window.window_state() == ferroui_controls::WindowState::Maximized,
    });
    state
}
