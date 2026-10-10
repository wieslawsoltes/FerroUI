//! Shared helpers of the tests: the application scope and the bodies of the generated
//! per-document tests.

use crate::{register_types, App, SAMPLE};
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use sample_testing::TestGlobalClock;
use std::rc::Rc;

/// Starts the application of the sample ([`App`]) as the application of the test, with the
/// test services of the samples in place of the services of a platform: the application
/// populates itself from `App.xaml` (the Fluent theme, the dock menu and the tray icon), as it
/// does when the application builder starts it.
pub fn start_application() -> UnitTestApplicationScope {
    register_types();
    UnitTestApplication::start_with(sample_testing::services(Rc::new(TestGlobalClock::default())), || App::new().upcast())
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
