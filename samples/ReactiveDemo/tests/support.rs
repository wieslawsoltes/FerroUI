//! Shared helpers of the tests: the application scope with ReactiveUI built, the bodies of the
//! generated per-document tests, a search service and a launcher that record what they are asked.

use crate::crate_search::{CatalogueSearchService, CrateSearchMetadata, ICrateSearchService};
use crate::{register_types, App, LauncherSource, SAMPLE};
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::{ILauncher, IStorageItem};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::Uri;
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use ferroui_reactiveui::{ReactiveUIBuilder, RxAppBuilder, RxSuspension};
use ferroui_reactiveui_controls::StartupConfiguration;
use ferroui_system::threading::tasks::Task;
use ferroui_system::threading::CancellationToken;
use sample_testing::TestGlobalClock;
use std::cell::RefCell;
use std::rc::Rc;

/// Starts the application of the sample ([`App`]) as the application of the test, with the test
/// services of the samples in place of the services of a platform, and builds ReactiveUI with the
/// services of FerroUI as `use_reactive_ui` of the entry point does.
pub fn start_application() -> UnitTestApplicationScope {
    register_types();
    let scope = UnitTestApplication::start_with(sample_testing::services(Rc::new(TestGlobalClock::default())), || App::new().upcast());
    build_reactive_ui();
    scope
}

/// Builds ReactiveUI on this thread from a clean state, with the services of FerroUI
/// (`use_reactive_ui`), and registers the views of the sample again: the reset replaces the
/// service locator the application registered them with.
pub fn build_reactive_ui() {
    RxAppBuilder::reset_for_testing();
    RxSuspension::reset_for_testing();
    StartupConfiguration::configure_reactive_ui(&|_: &ReactiveUIBuilder| {});
    crate::register_views();
}

/// `Dispatcher.UIThread.RunJobs()`.
pub fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
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

/// A search service over the in-memory catalogue that records the terms it is asked for.
#[derive(Default)]
pub struct RecordingSearchService {
    terms: RefCell<Vec<String>>,
}

impl RecordingSearchService {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The terms searched, in order.
    pub fn terms(&self) -> Vec<String> {
        self.terms.borrow().clone()
    }
}

impl ICrateSearchService for RecordingSearchService {
    fn search(&self, term: &str, token: CancellationToken) -> Task<Vec<CrateSearchMetadata>> {
        self.terms.borrow_mut().push(term.to_owned());
        CatalogueSearchService::new().search(term, token)
    }
}

/// A launcher that records the addresses it is asked to open.
#[derive(Default)]
pub struct RecordingLauncher {
    uris: RefCell<Vec<String>>,
}

impl RecordingLauncher {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The addresses opened, in order, as they were given.
    pub fn uris(&self) -> Vec<String> {
        self.uris.borrow().clone()
    }

    /// The launcher as the view models reach it.
    pub fn source(launcher: &Rc<RecordingLauncher>) -> LauncherSource {
        let launcher = launcher.clone();
        Rc::new(move || launcher.clone() as Rc<dyn ILauncher>)
    }
}

impl ILauncher for RecordingLauncher {
    fn launch_uri_async(&self, uri: &Uri) -> LocalBoxFuture<bool> {
        self.uris.borrow_mut().push(uri.original_string().to_owned());
        Box::pin(std::future::ready(true))
    }

    fn launch_file_async(&self, _storage_item: Rc<dyn IStorageItem>) -> LocalBoxFuture<bool> {
        Box::pin(std::future::ready(false))
    }
}

/// A crate of the catalogue by its name.
pub fn catalogue_crate(name: &str) -> CrateSearchMetadata {
    CatalogueSearchService::crates().into_iter().find(|krate| krate.name == name).unwrap_or_else(|| panic!("{name} is in the catalogue"))
}
