//! The shared helpers of the tests: the production application of the example as the application
//! of a test, the dispatcher's jobs and timers run until something happened, the awaiting of tasks,
//! temporary directories and the bodies of the generated per-document tests.
//!
//! Port of the test infrastructure of the reference (`ExampleTestSession.cs`,
//! `ExampleTestExecutor.cs`, `CallerTestExecutor.cs`): the reference starts one headless session of
//! the production application and runs every test on its UI thread, awaiting what the test awaits
//! through its dispatcher. Here each test starts the application on its own thread
//! ([`start_application`], or [`start_shell`] for a test that clicks), with the setup of ReactiveUI
//! the application builder makes (`ExampleAppBuilderExtensions::configure_reactive_ui`), and awaits
//! by running the jobs and the timers of the dispatcher ([`run_until`], [`await_task`]).

#![allow(dead_code)]

use crate::{register_types, App, ExampleAppBuilderExtensions, SAMPLE};
use ferroui_base::threading::Dispatcher;
use ferroui_base::BoxedValue;
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{RxAppBuilder, RxSuspension};
use ferroui_reactiveui_controls::{view_model_of_value, StartupConfiguration};
use ferroui_system::threading::tasks::{run_pending_wakes, Task};
use ferroui_system::{IDisposable, IObservable, ObservableError};
use sample_testing::{Shell, TestGlobalClock};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The longest a test waits for something to happen (the timeouts of the reference's tests).
pub const TIMEOUT: Duration = Duration::from_secs(5);

/// Starts the application of the example ([`App`]) as the application of the test, with the test
/// services of the samples in place of the services of a platform, and configures ReactiveUI as the
/// application builder does (`ExampleAppBuilderExtensions.Create().UseHeadless(..)` of the
/// reference's session).
pub fn start_application() -> UnitTestApplicationScope {
    register_types();
    let scope = UnitTestApplication::start_with(sample_testing::services(Rc::new(TestGlobalClock::default())), || App::new().upcast());
    configure_reactive_ui();
    scope
}

/// [`start_application`] on the headless shell of the samples, whose windows of `width` by `height`
/// render and take the input of a mouse.
pub fn start_shell(width: f64, height: f64) -> Shell {
    let shell = Shell::start(&SAMPLE, width, height, || App::new().upcast());
    configure_reactive_ui();
    shell
}

/// Builds ReactiveUI from a clean state with the setup of the example
/// (`UseShowcase()`, after the platform services are set up).
pub fn configure_reactive_ui() {
    RxAppBuilder::reset_for_testing();
    RxSuspension::reset_for_testing();
    StartupConfiguration::configure_reactive_ui(&ExampleAppBuilderExtensions::configure_reactive_ui);
}

/// The body of the generated test `document_<name>`.
pub fn document_loads(path: &str) {
    let _app = start_application();
    sample_testing::documents::document_loads(&SAMPLE, path);
}

/// The body of the generated test `class_<name>`.
pub fn class_constructs(path: &str) {
    let _app = start_application();
    sample_testing::documents::class_constructs(&SAMPLE, path);
}

/// `Dispatcher.UIThread.RunJobs()`.
pub fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

/// Fires every timer of the dispatcher of this thread now, then runs its jobs: the time a delay or
/// an interval waits for has passed.
pub fn fire_timers() {
    for timer in Dispatcher::timers_for_unit_tests() {
        Dispatcher::force_fire_timer_for_unit_tests(&timer);
    }
    run_jobs();
}

/// Runs the jobs of the dispatcher, the wakes of the tasks of this thread and the timers of the
/// dispatcher until `done` holds; whether it held within [`TIMEOUT`] (`WaitAsync(timeout)`).
pub fn run_until(mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        run_jobs();
        let _ = run_pending_wakes();
        if done() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        fire_timers();
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Awaits `task` on the thread of the test (`await task`): its result, or its error.
///
/// # Panics
/// Panics when the task has not ended within [`TIMEOUT`].
pub fn await_task<T: Clone + 'static>(task: &Task<T>) -> Result<T, ObservableError> {
    assert!(run_until(|| task.is_completed()), "the task did not end within the timeout");
    task.result()
}

/// Awaits the last value of `source` (`await source.ToTask()`).
pub fn await_last<T: Clone + 'static>(source: Rc<dyn IObservable<T>>) -> Result<T, ObservableError> {
    await_task(&source.to_task())
}

/// Whether two shared objects are the same object (`IsSameReferenceAs`).
pub fn same_object<T: ?Sized, U: ?Sized>(first: &Rc<T>, second: &Rc<U>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(first), Rc::as_ptr(second))
}

/// Whether the value of a property (a data context, a view model) holds the object `expected`.
pub fn holds_object<T: ?Sized>(value: &Option<BoxedValue>, expected: &Rc<T>) -> bool {
    value.as_ref().is_some_and(|value| same_object(&view_model_of_value(value), expected))
}

/// Disposes the object it holds when dropped (`using var x = ..`).
pub struct Disposing<T: IDisposable + ?Sized>(pub Rc<T>);

impl<T: IDisposable + ?Sized> Drop for Disposing<T> {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

/// A directory of the temporary folder with a name of its own, deleted with everything in it when
/// dropped (`Path.Combine(Path.GetTempPath(), $"..-{Guid.NewGuid():N}")` and the `finally` blocks
/// that delete it).
pub struct TemporaryDirectory {
    path: PathBuf,
}

impl TemporaryDirectory {
    /// A new name in the temporary folder; the directory is not created.
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|time| time.as_nanos()).unwrap_or_default();
        let name = format!("ReactiveUIExample-{}-{nanos}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed));
        Self { path: std::env::temp_dir().join(name) }
    }

    /// A new directory of the temporary folder, created.
    pub fn create() -> Self {
        let directory = Self::new();
        std::fs::create_dir_all(&directory.path).expect("the temporary directory is created");
        directory
    }

    /// The path of the directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The path of `name` in the directory, as text.
    pub fn file(&self, name: &str) -> String {
        self.path.join(name).to_string_lossy().into_owned()
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}
