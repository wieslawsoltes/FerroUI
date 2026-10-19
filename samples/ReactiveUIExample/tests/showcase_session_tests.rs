//! Port of `ShowcaseSessionTests.cs`: suspension persistence and deferral release against isolated
//! temporary session files.

use super::app_tests::SingleViewLifetimeProxy;
use super::support::{start_application, Disposing, TemporaryDirectory};
use crate::services::{LocalMachineMetricsService, ShowcaseSession};
use crate::view_models::MainViewModel;
use ferroui_controls::application_lifetimes::{ClassicDesktopStyleApplicationLifetime, IApplicationLifetime};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_system::{IDisposable, IObserver};
use std::cell::Cell;
use std::rc::Rc;

/// The prefix of the recoverable session storage errors (`StorageErrorPrefix`).
const STORAGE_ERROR_PREFIX: &str = "Session storage:";

/// The file name each test saves its session to (`SessionFileName`).
const SESSION_FILE_NAME: &str = "session.json";

/// A shell over the real local metrics service (`MainViewModel.Create(new LocalMachineMetricsService())`),
/// disposed at the end of the test.
fn create_shell() -> Disposing<MainViewModel> {
    Disposing(MainViewModel::create(Rc::new(LocalMachineMetricsService::new())))
}

/// A desktop lifetime (`new ClassicDesktopStyleApplicationLifetime()`).
fn desktop_lifetime() -> Rc<ClassicDesktopStyleApplicationLifetime> {
    ClassicDesktopStyleApplicationLifetime::new()
}

/// A deferral that records its release (`Scope.Create(released, state => state.Value = true)`).
fn deferral(released: &Rc<Cell<bool>>) -> Rc<dyn IDisposable> {
    let released = released.clone();
    Disposable::create(move || released.set(true))
}

#[test]
fn persistence_saves_and_next_launch_restores_settings() {
    const SAVED_INPUT: &str = "Inspect working set";
    const SAVED_THRESHOLD: f64 = 63.0;
    let _app = start_application();
    let directory = TemporaryDirectory::new();
    let path = directory.file(SESSION_FILE_NAME);

    {
        let shell = create_shell();
        let lifetime = desktop_lifetime();
        let session = Disposing(ShowcaseSession::new(&*lifetime as &dyn IApplicationLifetime, shell.0.clone(), Some(path.clone())));
        session.0.start();
        shell.0.commands().set_work_item_text(SAVED_INPUT.to_owned());
        shell.0.metrics().set_cpu_warning_threshold(SAVED_THRESHOLD);
        let released = Rc::new(Cell::new(false));
        session.0.persist(deferral(&released));
        assert!(released.get());
        assert!(std::path::Path::new(&path).exists());
        assert_eq!(shell.0.session_status(), "Session saved.");
    }

    {
        let shell = create_shell();
        let lifetime = desktop_lifetime();
        let session = Disposing(ShowcaseSession::new(&*lifetime as &dyn IApplicationLifetime, shell.0.clone(), Some(path.clone())));
        session.0.start();
        assert_eq!(shell.0.commands().work_item_text(), SAVED_INPUT);
        assert_eq!(shell.0.metrics().cpu_warning_threshold(), SAVED_THRESHOLD);
    }

    std::fs::write(&path, "Invalid JSON").expect("the session file is written");
    let recovered = create_shell();
    let lifetime = desktop_lifetime();
    let recovery = Disposing(ShowcaseSession::new(&*lifetime as &dyn IApplicationLifetime, recovered.0.clone(), Some(path)));
    recovery.0.start();
    assert!(recovered.0.session_status().starts_with(STORAGE_ERROR_PREFIX), "{}", recovered.0.session_status());
    assert_eq!(recovered.0.commands().work_item_text(), "");
}

#[test]
fn without_storage_or_controlled_lifetime_session_is_unavailable() {
    let _app = start_application();
    let single_view = SingleViewLifetimeProxy::create();
    let no_path_shell = create_shell();
    let lifetime = desktop_lifetime();
    let no_path = Disposing(ShowcaseSession::new(&*lifetime as &dyn IApplicationLifetime, no_path_shell.0.clone(), None));
    no_path.0.start();
    assert_eq!(no_path_shell.0.session_status(), ShowcaseSession::UNAVAILABLE_STATUS);

    let uncontrolled_shell = create_shell();
    let uncontrolled =
        Disposing(ShowcaseSession::new(&*single_view as &dyn IApplicationLifetime, uncontrolled_shell.0.clone(), Some("unused.json".to_owned())));
    uncontrolled.0.start();
    assert_eq!(uncontrolled_shell.0.session_status(), ShowcaseSession::UNAVAILABLE_STATUS);
}

#[test]
fn default_path_depends_on_platform() {
    assert!(ShowcaseSession::default_path_for(true).is_none());
    assert!(ShowcaseSession::default_path_for(false).is_some_and(|path| path.ends_with(SESSION_FILE_NAME)));
    assert_eq!(ShowcaseSession::default_path().is_some(), ShowcaseSession::default_path_for(cfg!(target_arch = "wasm32")).is_some());
}

#[test]
fn unhandled_exception_deletes_saved_session() {
    let _app = start_application();
    let directory = TemporaryDirectory::create();
    let path = directory.file(SESSION_FILE_NAME);
    std::fs::write(&path, "{}").expect("the session file is written");

    let shell = create_shell();
    let invalidation = Subject::<()>::new();
    let lifetime = desktop_lifetime();
    let _session = Disposing(ShowcaseSession::with_invalidation(
        &*lifetime as &dyn IApplicationLifetime,
        shell.0.clone(),
        Some(path.clone()),
        Some(invalidation.as_observable()),
    ));
    invalidation.on_next(());
    assert!(!std::path::Path::new(&path).exists());
    invalidation.dispose();
}

#[test]
fn empty_saved_session_reports_storage_error() {
    let _app = start_application();
    let directory = TemporaryDirectory::create();
    let path = directory.file(SESSION_FILE_NAME);
    std::fs::write(&path, "null").expect("the session file is written");

    let shell = create_shell();
    let lifetime = desktop_lifetime();
    let session = Disposing(ShowcaseSession::new(&*lifetime as &dyn IApplicationLifetime, shell.0.clone(), Some(path)));
    session.0.start();

    assert!(shell.0.session_status().starts_with(STORAGE_ERROR_PREFIX), "{}", shell.0.session_status());
}

#[test]
fn saved_session_without_input_restores_empty_input() {
    const SAVED_THRESHOLD: f64 = 42.0;
    let _app = start_application();
    let directory = TemporaryDirectory::create();
    let path = directory.file(SESSION_FILE_NAME);
    std::fs::write(&path, "{\"CpuWarningThreshold\":42}").expect("the session file is written");

    let shell = create_shell();
    shell.0.commands().set_work_item_text("Replaced by the restore".to_owned());
    let lifetime = desktop_lifetime();
    let session = Disposing(ShowcaseSession::new(&*lifetime as &dyn IApplicationLifetime, shell.0.clone(), Some(path)));
    session.0.start();

    assert_eq!(shell.0.commands().work_item_text(), "");
    assert_eq!(shell.0.metrics().cpu_warning_threshold(), SAVED_THRESHOLD);
}

#[test]
fn storage_failure_still_releases_shutdown_deferral() {
    let _app = start_application();
    let directory = TemporaryDirectory::create();
    let invalid_file = directory.file("directory-instead-of-file");
    std::fs::create_dir_all(&invalid_file).expect("the directory is created");

    let shell = create_shell();
    let lifetime = desktop_lifetime();
    let session = Disposing(ShowcaseSession::new(&*lifetime as &dyn IApplicationLifetime, shell.0.clone(), Some(invalid_file)));
    session.0.start();
    let released = Rc::new(Cell::new(false));
    session.0.persist(deferral(&released));

    assert!(released.get());
    assert!(shell.0.session_status().starts_with(STORAGE_ERROR_PREFIX), "{}", shell.0.session_status());
}

#[test]
fn invalidation_deletes_state_and_reports_storage_failure() {
    let _app = start_application();
    let directory = TemporaryDirectory::create();
    let path = directory.file(SESSION_FILE_NAME);
    let invalid_path = directory.file("directory-instead-of-file");
    std::fs::create_dir_all(&invalid_path).expect("the directory is created");
    std::fs::write(&path, "{}").expect("the session file is written");

    {
        let shell = create_shell();
        let lifetime = desktop_lifetime();
        let session = Disposing(ShowcaseSession::new(&*lifetime as &dyn IApplicationLifetime, shell.0.clone(), Some(path.clone())));
        session.0.invalidate();
        assert!(!std::path::Path::new(&path).exists());
    }

    let failed_shell = create_shell();
    let lifetime = desktop_lifetime();
    let failed_session = Disposing(ShowcaseSession::new(&*lifetime as &dyn IApplicationLifetime, failed_shell.0.clone(), Some(invalid_path.clone())));
    failed_session.0.invalidate();
    assert!(failed_shell.0.session_status().starts_with(STORAGE_ERROR_PREFIX), "{}", failed_shell.0.session_status());
    assert!(std::path::Path::new(&invalid_path).is_dir());
}
