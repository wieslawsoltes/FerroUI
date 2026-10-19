//! Port of `Services/ShowcaseSession.cs`: composes the suspension notifications of FerroUI with a
//! small local JSON settings store.

use crate::models::{ShowcaseJsonContext, ShowcaseState};
use crate::view_models::MainViewModel;
use ferroui_controls::application_lifetimes::IApplicationLifetime;
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::RxSuspension;
use ferroui_reactiveui_controls::AutoSuspendHelper;
use ferroui_system::{IDisposable, IObservable};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

/// The folder of the settings below the local application data folder (the name of the reference's
/// assembly, without the name of its framework).
const SETTINGS_FOLDER: &str = "ReactiveUIExample";

/// The file name of the settings.
const SETTINGS_FILE: &str = "session.json";

/// Composes the suspension notifications of FerroUI with a small local JSON settings store
/// (`ShowcaseSession`).
///
/// Deviation (DEVIATIONS.md, samples): the session is shared (`Rc`), as its subscriptions call it
/// back, and released with [`dispose`](IDisposable::dispose). A storage failure is any error of the
/// file system, where the reference catches `IOException` and `UnauthorizedAccessException`.
pub struct ShowcaseSession {
    /// The view model whose settings are persisted (`_shell`).
    shell: Rc<MainViewModel>,
    /// The settings file path (`_path`); empty, and never used, when the platform has no session
    /// storage.
    path: String,
    /// The platform lifetime adapter, or `None` when the session is not persisted (`_suspension`).
    suspension: Option<Rc<AutoSuspendHelper>>,
    /// The lifetime subscriptions (`_subscriptions`).
    subscriptions: Rc<CompositeDisposable>,
}

impl ShowcaseSession {
    /// The status shown when the platform has no session storage (`UnavailableStatus`).
    pub const UNAVAILABLE_STATUS: &'static str = "Session storage is not available on this platform. Input and threshold reset on each launch.";

    /// A session of `shell` saved to `path` (`ShowcaseSession(IApplicationLifetime, MainViewModel,
    /// string?)`). Only a controlled lifetime can persist a session; `None` for `path` when the
    /// platform has no session storage.
    pub fn new(lifetime: &dyn IApplicationLifetime, shell: Rc<MainViewModel>, path: Option<String>) -> Rc<Self> {
        Self::with_invalidation(lifetime, shell, path, None)
    }

    /// [`new`](Self::new) with the signal that saved state is invalid, or `None` to use the
    /// suspension host (the internal `ShowcaseSession(..., IObservable<Unit>?)`).
    pub fn with_invalidation(
        lifetime: &dyn IApplicationLifetime,
        shell: Rc<MainViewModel>,
        path: Option<String>,
        invalidation: Option<Rc<dyn IObservable<()>>>,
    ) -> Rc<Self> {
        let subscriptions = CompositeDisposable::new();
        let path = match path {
            Some(path) if lifetime.as_controlled_application_lifetime().is_some() => path,
            _ => return Rc::new(Self { shell, path: String::new(), suspension: None, subscriptions }),
        };

        let suspension = AutoSuspendHelper::new(Some(lifetime));
        let session = Rc::new(Self { shell, path, suspension: Some(suspension), subscriptions });
        let host = RxSuspension::suspension_host();

        let (on_launch, on_launch_error) = (Rc::downgrade(&session), Rc::downgrade(&session));
        session.subscriptions.add(host.is_launching_new().subscribe_next_error(
            move |_| with_session(&on_launch, |session| session.restore()),
            move |error| with_session(&on_launch_error, |session| session.report_failure(&error)),
        ));

        let (on_persist, on_persist_error) = (Rc::downgrade(&session), Rc::downgrade(&session));
        session.subscriptions.add(host.should_persist_state().subscribe_next_error(
            move |deferral: Rc<dyn IDisposable>| match on_persist.upgrade() {
                Some(session) => session.persist(deferral),
                None => deferral.dispose(),
            },
            move |error| with_session(&on_persist_error, |session| session.report_failure(&error)),
        ));

        let (on_invalidate, on_invalidate_error) = (Rc::downgrade(&session), Rc::downgrade(&session));
        let invalidation = invalidation.unwrap_or_else(|| host.should_invalidate_state());
        session.subscriptions.add(invalidation.subscribe_next_error(
            move |_| with_session(&on_invalidate, |session| session.invalidate()),
            move |error| with_session(&on_invalidate_error, |session| session.report_failure(&error)),
        ));

        session
    }

    /// The default settings file path, or `None` in a browser, which has no file storage
    /// (`DefaultPath()`).
    pub fn default_path() -> Option<String> {
        Self::default_path_for(cfg!(target_arch = "wasm32"))
    }

    /// The default settings file path for a platform (`DefaultPath(bool isBrowser)`): the file
    /// `ReactiveUIExample/session.json` of the local application data folder, `None` in a browser.
    pub fn default_path_for(is_browser: bool) -> Option<String> {
        if is_browser {
            return None;
        }
        let path = local_application_data().join(SETTINGS_FOLDER).join(SETTINGS_FILE);
        Some(path.to_string_lossy().into_owned())
    }

    /// Signals startup after the shell and its services have been composed (`Start()`).
    pub fn start(&self) {
        match &self.suspension {
            None => self.shell.set_session_status(Self::UNAVAILABLE_STATUS.to_owned()),
            Some(suspension) => suspension.on_framework_initialization_completed(),
        }
    }

    /// Saves the state and always releases the shutdown deferral (`Persist(IDisposable)`).
    pub fn persist(&self, deferral: Rc<dyn IDisposable>) {
        let _release = ReleaseOnDrop(deferral);
        match self.save() {
            Ok(()) => self.shell.set_session_status("Session saved.".to_owned()),
            Err(error) => self.report_failure(&error),
        }
    }

    /// Removes the settings after an unhandled application error (`Invalidate()`). A missing file
    /// is not an error, as `File.Delete` of the reference does not report one.
    pub fn invalidate(&self) {
        match fs::remove_file(&self.path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => self.report_failure(&error),
        }
    }

    /// Writes the state to a temporary file, then moves it over the settings file.
    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let state = ShowcaseState::new(self.shell.commands().work_item_text(), self.shell.metrics().cpu_warning_threshold());
        let full_path = std::path::absolute(Path::new(&self.path))?;
        if let Some(directory) = full_path.parent() {
            fs::create_dir_all(directory)?;
        }
        let temporary_path = format!("{}.tmp", self.path);
        fs::write(&temporary_path, ShowcaseJsonContext::serialize_showcase_state(&state)?)?;
        fs::rename(&temporary_path, &self.path)?;
        Ok(())
    }

    /// Restores the last session or reports a recoverable storage error (`Restore()`).
    fn restore(&self) {
        if !Path::new(&self.path).exists() {
            self.shell.set_session_status("New session. Input and threshold save on exit.".to_owned());
            return;
        }

        match self.load() {
            Ok(state) => {
                self.shell.commands().set_work_item_text(state.work_item_text.unwrap_or_default());
                self.shell.metrics().set_cpu_warning_threshold(state.cpu_warning_threshold);
                self.shell.set_session_status("Previous input and threshold restored.".to_owned());
            }
            Err(error) => self.report_failure(&error),
        }
    }

    /// Reads the saved state; an error when the document is `null` (`JsonException("The saved
    /// session is empty.")`).
    fn load(&self) -> Result<ShowcaseState, Box<dyn std::error::Error>> {
        let text = fs::read_to_string(&self.path)?;
        let state = ShowcaseJsonContext::deserialize_showcase_state(&text)?;
        state.ok_or_else(|| "The saved session is empty.".into())
    }

    /// Surfaces storage failures in the shell (`ReportFailure`).
    fn report_failure(&self, error: &dyn fmt::Display) {
        self.shell.set_session_status(format!("Session storage: {error}"));
    }
}

impl IDisposable for ShowcaseSession {
    /// `Dispose()`.
    fn dispose(&self) {
        self.subscriptions.dispose();
        if let Some(suspension) = &self.suspension {
            suspension.dispose();
        }
    }
}

/// Runs `action` on the session while it is alive.
fn with_session(session: &Weak<ShowcaseSession>, action: impl FnOnce(&ShowcaseSession)) {
    if let Some(session) = session.upgrade() {
        action(&session);
    }
}

/// Disposes the deferral it holds when dropped (`using (deferral)`).
struct ReleaseOnDrop(Rc<dyn IDisposable>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

/// The local application data folder (`Environment.SpecialFolder.LocalApplicationData`): the
/// folder `%LOCALAPPDATA%` on Windows, `~/Library/Application Support` on macOS and iOS, and
/// `$XDG_DATA_HOME` or `~/.local/share` elsewhere; empty when the variables it is read from are
/// not set, as the reference's folder is.
fn local_application_data() -> PathBuf {
    let variable = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty()).map(PathBuf::from);
    if cfg!(windows) {
        return variable("LOCALAPPDATA").unwrap_or_default();
    }
    if cfg!(any(target_os = "macos", target_os = "ios")) {
        return variable("HOME").map(|home| home.join("Library").join("Application Support")).unwrap_or_default();
    }
    if let Some(data_home) = variable("XDG_DATA_HOME").filter(|path| path.is_absolute()) {
        return data_home;
    }
    variable("HOME").map(|home| home.join(".local").join("share")).unwrap_or_default()
}
