//! Opt-in native two-window verification using the unchanged compiled sample views.
//! All settings/documents are held in memory; no user storage or clipboard is read.
use ferroboard_sample::{
    App,
    composition::SampleSession,
    sample_options::SampleOptions,
    sample_storage::{SETTINGS_KEY, SampleStorage, SampleStoragePlatform, StorageResult},
};
use ferroui_base::{
    FerroObjectImpl, PixelPoint,
    threading::{DispatcherPriority, DispatcherTimer},
};
use ferroui_controls::{
    AppBuilder, ApplicationImpl, ApplicationImplExt, NewApplication,
    application_lifetimes::ClassicDesktopStyleApplicationLifetime,
};
use ferroui_desktop::AppBuilderDesktopExtensions;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    error::Error,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

struct VolatileStorage {
    settings: RefCell<HashMap<String, String>>,
    documents: RefCell<HashMap<PathBuf, String>>,
}
impl VolatileStorage {
    fn new() -> Self {
        Self {
            settings: RefCell::new(HashMap::from([(
                SETTINGS_KEY.to_owned(),
                r#"{"learnWords":false,"emojiHistoryEnabled":false,"clipboardHistoryEnabled":false,"clipboardSuggestion":false}"#.to_owned(),
            )])),
            documents: RefCell::new(HashMap::new()),
        }
    }
}
impl SampleStoragePlatform for VolatileStorage {
    fn get_setting(&self, key: &str) -> StorageResult<Option<String>> {
        Ok(self.settings.borrow().get(key).cloned())
    }
    fn set_setting(&self, key: &str, value: &str) -> StorageResult<()> {
        self.settings
            .borrow_mut()
            .insert(key.to_owned(), value.to_owned());
        Ok(())
    }
    fn local_folder(&self) -> StorageResult<Option<PathBuf>> {
        Ok(None)
    }
    fn read_document(&self, name: &Path) -> StorageResult<Option<String>> {
        Ok(self.documents.borrow().get(name).cloned())
    }
    fn write_document(&self, name: &Path, content: &str) -> StorageResult<()> {
        self.documents
            .borrow_mut()
            .insert(name.to_owned(), content.to_owned());
        Ok(())
    }
    fn log(&self, message: &str) {
        eprintln!("window focus probe: {message}");
    }
}

#[derive(Default)]
struct ProbeState {
    sessions: Vec<Option<Rc<SampleSession>>>,
    events: VecDeque<String>,
}
type Sessions = Rc<RefCell<ProbeState>>;
fn note(state: &Sessions, event: String) {
    let mut state = state.borrow_mut();
    state.events.push_back(event);
    while state.events.len() > 6 {
        state.events.pop_front();
    }
}

#[repr(C)]
struct ProbeApp {
    base: App,
}
ferroui_base::ferro_class!(ProbeApp: App);
ferroui_base::ferro_impl_classes!(ProbeApp: FerroObjectImpl);
impl NewApplication for ProbeApp {
    fn new_application() -> ferroui_base::Ref<Self> {
        let app = ferroui_base::instantiate(Self {
            base: App::construct(),
        });
        app.set_name(Some("FerroBoard Window Check".into()));
        app
    }
}
impl ApplicationImpl for ProbeApp {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
    }
    fn on_framework_initialization_completed(_this: &Self) {
        // The probe composes two volatile sessions below instead of the normal file session.
    }
}

fn refresh_titles(sessions: &Sessions, retired_keyboard_alive: Option<bool>) {
    // Never retain a RefCell guard across native notifications.
    let (snapshot, events) = {
        let state = sessions.borrow();
        (
            state
                .sessions
                .iter()
                .cloned()
                .enumerate()
                .collect::<Vec<_>>(),
            state.events.iter().cloned().collect::<Vec<_>>().join(","),
        )
    };
    let live = snapshot
        .iter()
        .filter(|(_, session)| session.is_some())
        .count();
    // An automation snapshot may represent an inactive peer. Include BOTH native roots so
    // an empty target on the observed window is not mistaken for failed peer activation.
    let focus = snapshot
        .iter()
        .filter_map(|(index, session)| {
            session.as_ref().map(|session| {
                let scope = session
                    .window
                    .focus_manager()
                    .get_focused_element_in_scope(&session.window)
                    .and_then(|e| e.name());
                let target = session.keyboard.target_element().and_then(|e| e.name());
                format!(
                    "{} active={} scope={} target={}",
                    ['A', 'B'][*index],
                    session.window.is_active(),
                    scope.as_deref().unwrap_or("none"),
                    target.as_deref().unwrap_or("none")
                )
            })
        })
        .collect::<Vec<_>>()
        .join(" | ");
    for (index, session) in snapshot {
        if let Some(session) = session {
            if live == 1 {
                let actions = session
                    .keyboard
                    .smartbar_actions()
                    .expect("probe action vector");
                let removed = {
                    let mut actions = actions.lock().expect("probe action vector lock");
                    let count = actions.len();
                    actions.retain(|action| {
                        action.as_ref().and_then(|a| a.id.as_ref())
                            != Some(&"probe_activate_peer".into())
                    });
                    count != actions.len()
                };
                if removed {
                    session
                        .keyboard
                        .apply_theme()
                        .expect("refresh probe actions after peer close");
                }
            }
            let target = session.keyboard.target_element().and_then(|e| e.name());
            let retired = retired_keyboard_alive.map_or(String::new(), |alive| {
                format!("; retired keyboard alive={alive}")
            });
            session.window.set_title(Some(format!(
                "FerroBoard {} — target={}; windows={live}{retired} [{focus}] events={events}",
                ['A', 'B'][index],
                target.as_deref().unwrap_or("none"),
            )));
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).unwrap_or_else(|panic| {
            let message = panic
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied())
                .unwrap_or("non-string panic");
            Err(format!("probe panicked: {message}").into())
        });
    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            // Native LaunchServices does not expose stderr to the UI driver. Keep startup
            // failure visible in a diagnostic window; closing it still returns failure.
            eprintln!("window focus probe failed: {error}");
            let window = ferroui_controls::Window::new();
            window.set_title(Some(format!("FerroBoard probe FAILED: {error}")));
            window.set_width(740.0);
            window.set_height(250.0);
            let message = ferroui_controls::TextBlock::new();
            message.set_text(Some(&error.to_string()));
            window.set_content(Some(ferroui_controls::Control::boxed(message)));
            let closed = window.closed(|| {
                ferroui_base::threading::Dispatcher::ui_thread()
                    .begin_invoke_shutdown(DispatcherPriority::NORMAL);
            });
            window.show();
            ferroui_base::threading::Dispatcher::ui_thread()
                .main_loop(&ferroui_system::threading::CancellationToken::none());
            closed.dispose();
            Err(error)
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    AppBuilder::configure::<ProbeApp>()
        .use_platform_detect()
        .setup_with_lifetime(lifetime.clone());
    let sessions: Sessions = Rc::new(RefCell::new(ProbeState::default()));
    let mut subscriptions: Vec<ferroui_system::Subscription> = Vec::new();
    for index in 0..2 {
        let options = Arc::new(SampleOptions::default());
        options.parse(Some(&"--settings=false".into()));
        let session = SampleSession::with_options(
            SampleStorage::new(Rc::new(VolatileStorage::new())),
            options,
        )?;
        let state = Rc::downgrade(&sessions);
        let engine = session.keyboard.engine();
        let observed_engine = Rc::downgrade(&engine);
        let token = engine.text_committed().add(Rc::new(move |text| {
            let Some(engine) = observed_engine.upgrade() else {
                return;
            };
            let Some(target) = engine.target() else {
                return;
            };
            let attributes = target.attributes();
            if attributes.is_private()
                || attributes.is_password()
                || engine.settings().incognito_mode()
            {
                return;
            }
            if let Some(state) = state.upgrade() {
                // Record unit counts only, never text, passwords or word history.
                note(
                    &state,
                    format!("{}commit{}", ['A', 'B'][index], text.units().len()),
                );
                refresh_titles(&state, None);
            }
        }));
        let engine = Rc::downgrade(&engine);
        subscriptions.push(
            ferroui_base::reactive::Disposable::create(move || {
                if let Some(engine) = engine.upgrade() {
                    engine.text_committed().remove(token);
                }
            })
            .into(),
        );
        for activated in [true, false] {
            let state = Rc::downgrade(&sessions);
            let report = move || {
                let state = state.clone();
                // Window.Activated is raised before native scope restoration. Observe after
                // that dispatch, without focusing fields or scheduling any engine operation.
                ferroui_base::threading::Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(state) = state.upgrade() {
                            refresh_titles(&state, None);
                        }
                    },
                    DispatcherPriority::BACKGROUND,
                );
            };
            subscriptions.push(if activated {
                session.window.activated(report)
            } else {
                session.window.deactivated(report)
            });
        }
        session.window.set_width(740.0);
        session.window.set_height(760.0);
        session.window.set_position(PixelPoint::new(
            30 + index as i32 * 330,
            70 + index as i32 * 70,
        ));

        let state = Rc::downgrade(&sessions);
        let token = session.keyboard.target_changed().add(Rc::new(move |_| {
            if let Some(state) = state.upgrade() {
                refresh_titles(&state, None);
            }
        }));
        let keyboard = session.keyboard.downgrade();
        subscriptions.push(
            ferroui_base::reactive::Disposable::create(move || {
                if let Some(keyboard) = keyboard.upgrade() {
                    keyboard.target_changed().remove(token);
                }
            })
            .into(),
        );

        let state = Rc::downgrade(&sessions);
        let keyboard = session.keyboard.downgrade();
        subscriptions.push(session.window.closed(move || {
            if let Some(state) = state.upgrade() {
                let retired = state.borrow_mut().sessions[index].take();
                drop(retired);
                let state = Rc::downgrade(&state);
                let keyboard = keyboard.clone();
                // Observe lifetime after the native Closed dispatch unwinds. This timer only
                // reports probe state; it never schedules engine or input operations.
                DispatcherTimer::run_once(
                    move || {
                        if let Some(state) = state.upgrade() {
                            refresh_titles(&state, Some(keyboard.upgrade().is_some()));
                        }
                    },
                    Duration::from_millis(50),
                    DispatcherPriority::NORMAL,
                )
                .detach();
            }
        }));
        sessions.borrow_mut().sessions.push(Some(session));
    }
    for index in 0..2 {
        let session = sessions.borrow().sessions[index]
            .clone()
            .expect("live probe session");
        let peer = 1 - index;
        let state = Rc::downgrade(&sessions);
        session
            .keyboard
            .smartbar_actions()
            .ok_or("sample keyboard actions are unavailable")?
            .lock()
            .map_err(|_| "sample keyboard actions lock is poisoned")?
            .push(Some(ferroboard_controls::SmartbarAction::new(
                Some("probe_activate_peer".into()),
                ferroboard_core::keys::KeyIcon::None,
                Some(
                    format!("Activate window {}", ['A', 'B'][peer])
                        .as_str()
                        .into(),
                ),
                Some(Rc::new(move |_| {
                    let state = state.upgrade().expect("probe state retired");
                    let target = state.borrow().sessions[peer]
                        .clone()
                        .expect("peer window is closed");
                    note(
                        &state,
                        format!("{}>{}", ['A', 'B'][index], ['A', 'B'][peer]),
                    );
                    target.window.activate();
                    refresh_titles(&state, None);
                    Ok(())
                })),
                None,
            )));
    }
    refresh_titles(&sessions, None);
    let windows: Vec<_> = sessions
        .borrow()
        .sessions
        .iter()
        .flatten()
        .map(|s| s.window.clone())
        .collect();
    for window in windows {
        window.show();
    }
    let exit_code = lifetime.start(&[]);
    for subscription in subscriptions {
        subscription.dispose();
    }
    lifetime.dispose();
    if exit_code != 0 {
        return Err(format!("native desktop lifetime exited with {exit_code}").into());
    }
    Ok(())
}
