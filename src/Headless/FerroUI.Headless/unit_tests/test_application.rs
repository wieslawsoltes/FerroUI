//! Port of `TestApplication.cs` of the upstream unit test project of the
//! headless platform, with what stands for the test projects that compile
//! its files and for the test framework integrations that run them.
//!
//! Upstream compiles the test files into four projects: for xUnit and for
//! NUnit, each with the `PerTest` and with the `PerAssembly` isolation, and
//! the `PerTest` projects alone share the mouse device, "so that both mouse
//! device modes are covered". Here the two isolations are two
//! [`FerroTestAssembly`] values of this crate, and every test runs once in
//! each (`ferro_fact!`, `ferro_theory!`): `per_test` and `per_assembly`
//! are the two tests of a test function.
//!
//! The test framework integrations (the facts and theories that dispatch a
//! test to the session of its assembly) are not ported, there being no test
//! framework to integrate with: [`run`] and [`run_async`] do what the test
//! runner of the xUnit integration does.

use crate::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute, FerroTestAssembly,
    FerroTestIsolationAttribute, FerroTestIsolationLevel, HeadlessUnitTestSession, HeadlessUnitTestTask,
};
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{AppBuilder, Application, ApplicationImpl, NewApplication};
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_skia::SkiaApplicationExtensions;
use ferroui_themes_simple::SimpleTheme;
use std::cell::Cell;
use std::collections::HashMap;
use std::future::Future;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, Once, OnceLock, PoisonError};

#[repr(C)]
pub(crate) struct TestApplication {
    base: Application,
}

ferro_class!(TestApplication: Application);
ferro_impl_classes!(TestApplication: FerroObjectImpl);

impl NewApplication for TestApplication {
    fn new_application() -> Ref<Self> {
        let this = instantiate(Self { base: Application::construct() });
        this.styles().add(SimpleTheme::new().as_style());
        this
    }
}

impl ApplicationImpl for TestApplication {}

thread_local! {
    /// The isolation of the test assembly whose session runs on this thread: the compilation
    /// symbol of the original (`PERTEST`).
    static ISOLATION_LEVEL: Cell<FerroTestIsolationLevel> = const { Cell::new(FerroTestIsolationLevel::PerTest) };
}

impl TestApplication {
    /// The isolation level of the assembly of the running test: what the tests of the original
    /// read from the attribute of their assembly.
    pub(crate) fn isolation_level() -> FerroTestIsolationLevel {
        ISOLATION_LEVEL.get()
    }

    /// Enabled by the PerTest projects only, so that both mouse device modes are covered.
    pub(crate) fn uses_shared_mouse_device() -> bool {
        Self::isolation_level() == FerroTestIsolationLevel::PerTest
    }

    pub(crate) fn build_ferro_app() -> AppBuilder {
        AppBuilder::configure::<TestApplication>().use_harfbuzz().use_skia().use_headless(FerroHeadlessPlatformOptions {
            use_headless_drawing: false,
            overlay_popups: false,
            use_shared_mouse_device: Some(Self::uses_shared_mouse_device()),
            ..FerroHeadlessPlatformOptions::default()
        })
    }
}

fn build_per_test_app() -> AppBuilder {
    ISOLATION_LEVEL.set(FerroTestIsolationLevel::PerTest);
    TestApplication::build_ferro_app()
}

fn build_per_assembly_app() -> AppBuilder {
    ISOLATION_LEVEL.set(FerroTestIsolationLevel::PerAssembly);
    TestApplication::build_ferro_app()
}

/// The assembly information of the `PerTest` test projects.
pub(crate) static PER_TEST: FerroTestAssembly = FerroTestAssembly {
    name: "ferroui-headless-per-test-unit-tests",
    test_application: Some(FerroTestApplicationAttribute::new(build_per_test_app)),
    test_isolation: Some(FerroTestIsolationAttribute::new(FerroTestIsolationLevel::PerTest)),
};

/// The assembly information of the `PerAssembly` test projects.
pub(crate) static PER_ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferroui-headless-per-assembly-unit-tests",
    test_application: Some(FerroTestApplicationAttribute::new(build_per_assembly_app)),
    test_isolation: Some(FerroTestIsolationAttribute::new(FerroTestIsolationLevel::PerAssembly)),
};

thread_local! {
    /// The number of the test that the session of this thread runs, or ran last.
    static CURRENT_TEST: Cell<u64> = const { Cell::new(0) };
}

static NEXT_TEST: AtomicU64 = AtomicU64::new(1);

/// The messages of the panics of the session threads, by test. A session thread is not the
/// thread of the test, so what its panic hook prints is not in the output of the failed test:
/// the test panics again with the message and the place.
fn session_panics() -> &'static Mutex<HashMap<u64, String>> {
    static PANICS: OnceLock<Mutex<HashMap<u64, String>>> = OnceLock::new();
    static HOOK: Once = Once::new();
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let test = CURRENT_TEST.try_with(Cell::get).unwrap_or(0);
            if test != 0 {
                let mut panics = session_panics().lock().unwrap_or_else(PoisonError::into_inner);
                // Every panic of the test, also the ones that were caught (the render loop
                // catches the panic of a tick).
                let messages = panics.entry(test).or_default();
                if !messages.is_empty() {
                    messages.push_str("\nthen ");
                }
                messages.push_str(&info.to_string());
            }
            previous(info);
        }));
    });
    PANICS.get_or_init(Mutex::default)
}

/// Waits for a dispatched test and fails as it failed.
fn wait(task: HeadlessUnitTestTask<()>, test: u64) {
    match catch_unwind(AssertUnwindSafe(|| task.wait())) {
        Ok(result) => result.expect("the test was cancelled"),
        Err(payload) => {
            let message = session_panics().lock().unwrap_or_else(PoisonError::into_inner).remove(&test);
            match message {
                Some(message) => panic!("{message}"),
                None => resume_unwind(payload),
            }
        }
    }
}

/// Runs a test on the dispatcher thread of the session of `assembly`, as the test runner of
/// the xUnit integration does: the test, then the jobs of the dispatcher.
pub(crate) fn run(assembly: &'static FerroTestAssembly, test: impl FnOnce() + Send + 'static) {
    session_panics();
    let session = HeadlessUnitTestSession::get_or_start_for_assembly(Some(assembly));
    let number = NEXT_TEST.fetch_add(1, Ordering::Relaxed);
    let task = session.dispatch(
        move || {
            CURRENT_TEST.set(number);
            let dispatcher = Dispatcher::ui_thread();
            test();
            dispatcher.run_jobs(None);
        },
        CancellationToken::none(),
    );
    wait(task, number);
}

/// [`run`] for an asynchronous test.
pub(crate) fn run_async<Fut: Future<Output = ()> + 'static>(
    assembly: &'static FerroTestAssembly,
    test: impl FnOnce() -> Fut + Send + 'static,
) {
    session_panics();
    let session = HeadlessUnitTestSession::get_or_start_for_assembly(Some(assembly));
    let number = NEXT_TEST.fetch_add(1, Ordering::Relaxed);
    let task = session.dispatch_task(
        move || {
            CURRENT_TEST.set(number);
            async move {
                let dispatcher = Dispatcher::ui_thread();
                test().await;
                dispatcher.run_jobs(None);
            }
        },
        CancellationToken::none(),
    );
    wait(task, number);
}

/// The fact of the test framework integrations: the test function of the name runs on the
/// session of each of the two test assemblies.
macro_rules! ferro_fact {
    ($(#[$attribute:meta])* $name:ident) => {
        $crate::unit_tests::test_application::ferro_fact!(@tests run, $name, [$(#[$attribute])*], []);
    };
    ($(#[$attribute:meta])* async $name:ident) => {
        $crate::unit_tests::test_application::ferro_fact!(@tests run_async, $name, [$(#[$attribute])*], []);
    };
    (@tests $run:ident, $name:ident, [$(#[$attribute:meta])*], [$(#[$shared:meta])*]) => {
        mod $name {
            use $crate::unit_tests::test_application::{$run, PER_ASSEMBLY, PER_TEST};

            #[test]
            $(#[$attribute])*
            fn per_test() {
                $run(&PER_TEST, super::$name);
            }

            #[test]
            $(#[$attribute])*
            $(#[$shared])*
            fn per_assembly() {
                $run(&PER_ASSEMBLY, super::$name);
            }
        }
    };
}

/// The theory of the test framework integrations: one test per row and test assembly, each a
/// dispatch of its own.
macro_rules! ferro_theory {
    ($name:ident { $($row:ident: ($($argument:expr),*)),* $(,)? }) => {
        $crate::unit_tests::test_application::ferro_theory!(@tests run, $name, [], $($row: ($($argument),*)),*);
    };
    (async $name:ident { $($row:ident: ($($argument:expr),*)),* $(,)? }) => {
        $crate::unit_tests::test_application::ferro_theory!(@tests run_async, $name, [], $($row: ($($argument),*)),*);
    };
    (@tests $run:ident, $name:ident, $shared:tt, $($row:ident: ($($argument:expr),*)),*) => {
        mod $name {
            mod per_test {
                use $crate::unit_tests::test_application::{$run, PER_TEST};

                $(
                    #[test]
                    fn $row() {
                        $run(&PER_TEST, || super::super::$name($($argument),*));
                    }
                )*
            }

            mod per_assembly {
                use $crate::unit_tests::test_application::{$run, PER_ASSEMBLY};

                $(
                    $crate::unit_tests::test_application::ferro_theory!(@row $run, $name, $shared, $row: ($($argument),*));
                )*
            }
        }
    };
    (@row $run:ident, $name:ident, [$(#[$shared:meta])*], $row:ident: ($($argument:expr),*)) => {
        #[test]
        $(#[$shared])*
        fn $row() {
            $run(&PER_ASSEMBLY, || super::super::$name($($argument),*));
        }
    };
}

pub(crate) use {ferro_fact, ferro_theory};
