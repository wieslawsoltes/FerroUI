//! The scenarios of the crate browser: the application is mounted on the test application of
//! `ferroui-controls` (a dispatcher of the thread) with the in-memory catalogue of crates (or a
//! service of the test), its controls are found by `Name` and driven as a user would (typing
//! sets the text of a field, a click raises `Click`). The throttle of the search is a timer of the
//! dispatcher: [`Ui::settle_until`] fires the timers that are due later and runs what they woke,
//! until what the test waits for is shown.

use crate::view::{app, Browser};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ObjectType, Ref, StyledElement};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Button, Control, Panel, TextBlock, TextBox};
use ferroui_system::threading::tasks::{run_pending_wakes, Task};
use ferroui_system::threading::CancellationToken;
use ferroui_view::{Element, IntoElement, Owner};
use reactive_demo::crate_search::{CatalogueSearchService, CrateSearchMetadata, ICrateSearchService};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// The hint of a search that matches no crate.
const NO_MATCH: &str = "No crate matches the search.";

/// The mounted application, disposed when it is dropped.
struct Ui {
    root: Ref<Control>,
    owner: Option<Owner>,
    _app: UnitTestApplicationScope,
}

impl Ui {
    /// Mounts the application on `path`, searching with `service`, with a browser the test keeps.
    fn app_on(path: &str, service: Rc<dyn ICrateSearchService>) -> (Ui, Browser) {
        let scope = UnitTestApplication::start(TestServices::mock_threading_interface());
        let owner = Owner::new_root();
        let (root, browser) = owner.with(|| {
            let browser = Browser::new(service);
            let element: Element = app().browser(Some(browser.clone())).path(path).into_element();
            (element.mount(), browser)
        });
        let ui = Ui { root, owner: Some(owner), _app: scope };
        ui.run_jobs();
        (ui, browser)
    }

    /// Mounts the application on `/` with the in-memory catalogue.
    fn app() -> (Ui, Browser) {
        Self::app_on("/", Rc::new(CatalogueSearchService::new()))
    }

    /// Runs the work queued on the dispatcher and the futures woken from other threads.
    fn run_jobs(&self) {
        Dispatcher::ui_thread().run_jobs(None);
        run_pending_wakes();
    }

    /// Lets time pass until `done` holds: fires the timers of the dispatcher (the throttle of the
    /// search) and runs what they woke.
    ///
    /// # Panics
    /// Panics when `done` does not hold within ten seconds.
    fn settle_until(&self, what: &str, done: impl Fn(&Ui) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            self.run_jobs();
            if done(self) {
                return;
            }
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            let timers = Dispatcher::timers_for_unit_tests();
            if timers.is_empty() {
                // Nothing is due on the dispatcher: what is awaited comes from elsewhere.
                std::thread::sleep(Duration::from_millis(5));
            }
            for timer in &timers {
                Dispatcher::force_fire_timer_for_unit_tests(timer);
            }
        }
    }

    /// The control named `name`, of class `T`, if there is one.
    fn try_find<T: ObjectType>(&self, name: &str) -> Option<Ref<T>> {
        fn search(element: &Ref<StyledElement>, name: &str) -> Option<Ref<StyledElement>> {
            if element.name().as_deref() == Some(name) {
                return Some(element.clone());
            }
            element.logical_children().to_vec().iter().find_map(|child| search(child, name))
        }
        let root: Ref<StyledElement> = self.root.clone().upcast();
        search(&root, name).and_then(|found| found.cast::<T>())
    }

    /// The control named `name`, of class `T`.
    ///
    /// # Panics
    /// Panics when there is none.
    fn find<T: ObjectType>(&self, name: &str) -> Ref<T> {
        self.try_find(name).unwrap_or_else(|| panic!("no {} named `{name}`", std::any::type_name::<T>()))
    }

    /// Whether a control is named `name`.
    fn has(&self, name: &str) -> bool {
        self.try_find::<Control>(name).is_some()
    }

    /// Clicks the button named `name`.
    fn click(&self, name: &str) {
        let button = self.find::<Button>(name);
        assert!(button.is_effectively_enabled(), "`{name}` is enabled");
        button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        self.run_jobs();
    }

    /// Types `text` into the field named `name`.
    fn type_text(&self, name: &str, text: &str) {
        self.find::<TextBox>(name).set_text(Some(text));
        self.run_jobs();
    }

    /// The text of the field or text block named `name`.
    fn text(&self, name: &str) -> String {
        let control = self.find::<Control>(name);
        if let Some(text_box) = control.cast::<TextBox>() {
            return text_box.text().unwrap_or_default();
        }
        control.cast::<TextBlock>().and_then(|text| text.text()).unwrap_or_default()
    }

    /// The names of the crates of the results, in order.
    fn results(&self) -> Vec<String> {
        self.find::<Panel>("Results")
            .children()
            .to_vec()
            .iter()
            .filter_map(|result| result.name())
            .map(|name| name.trim_start_matches("Result-").to_string())
            .collect()
    }

    /// Searches `term` (not blank) and waits for what the search gives: results, the hint that
    /// nothing matches, or an error.
    fn search(&self, term: &str) {
        self.type_text("Search", term);
        self.settle_until("the outcome of the search", |ui| {
            !ui.has("Searching")
                && (ui.has("Results") || ui.has("Error") || (ui.has("Hint") && ui.text("Hint") == NO_MATCH))
        });
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        if let Some(owner) = self.owner.take() {
            owner.dispose();
        }
    }
}

/// A search over the in-memory catalogue that records the terms it is asked for.
#[derive(Default)]
struct RecordingSearchService {
    terms: RefCell<Vec<String>>,
}

impl ICrateSearchService for RecordingSearchService {
    fn search(&self, term: &str, token: CancellationToken) -> Task<Vec<CrateSearchMetadata>> {
        self.terms.borrow_mut().push(term.to_owned());
        CatalogueSearchService::new().search(term, token)
    }
}

/// A search that always fails, as one without a network does.
struct FailingSearchService;

impl ICrateSearchService for FailingSearchService {
    fn search(&self, _term: &str, _token: CancellationToken) -> Task<Vec<CrateSearchMetadata>> {
        Task::from_exception(Rc::new(std::io::Error::other("crates.io is not reachable")))
    }
}

/// A crate of the catalogue by its name.
fn catalogue_crate(name: &str) -> CrateSearchMetadata {
    CatalogueSearchService::crates()
        .into_iter()
        .find(|krate| krate.name == name)
        .unwrap_or_else(|| panic!("{name} is in the catalogue"))
}

#[test]
fn an_empty_search_shows_a_hint_and_no_results() {
    let (ui, _browser) = Ui::app();
    ui.settle_until("the hint", |ui| ui.has("Hint"));
    assert!(!ui.has("Results") && !ui.has("Searching"));
    ui.type_text("Search", "   ");
    ui.settle_until("the hint", |ui| ui.has("Hint"));
    assert!(!ui.has("Results"), "a blank text is no search");
}

#[test]
fn a_search_waits_for_the_throttle_then_shows_its_results() {
    let (ui, _browser) = Ui::app();
    ui.type_text("Search", "http client");
    assert!(ui.has("Searching"), "the search is pending while the throttle runs");
    assert_eq!(ui.text("Status"), "Searching...");
    assert!(!ui.has("Results"));

    ui.settle_until("the results", |ui| ui.has("Results"));
    assert!(!ui.has("Searching"));
    assert_eq!(ui.results(), ["reqwest", "hyper"]);

    ui.type_text("Search", "");
    ui.settle_until("the hint", |ui| ui.has("Hint"));
    assert!(!ui.has("Results"), "an empty text shows no results");
}

#[test]
fn only_the_text_the_user_stopped_at_is_searched() {
    let service = Rc::new(RecordingSearchService::default());
    let (ui, _browser) = Ui::app_on("/", service.clone());
    ui.type_text("Search", "se");
    ui.type_text("Search", "ser");
    ui.type_text("Search", "  serde  ");
    ui.settle_until("the results", |ui| ui.has("Results"));
    assert_eq!(*service.terms.borrow(), ["serde"], "the terms typed past are dropped while they wait, and the term is trimmed");
    assert_eq!(ui.results(), ["serde"]);

    // The same term, trimmed, is not searched again.
    ui.type_text("Search", "serde ");
    ui.settle_until("the results", |ui| ui.has("Results"));
    assert_eq!(service.terms.borrow().len(), 1);
}

#[test]
fn a_search_that_matches_nothing_says_so() {
    let (ui, _browser) = Ui::app();
    ui.search("no such crate anywhere");
    assert_eq!(ui.text("Hint"), NO_MATCH);
    assert!(!ui.has("Results"));
}

#[test]
fn a_failed_search_shows_its_error() {
    let (ui, _browser) = Ui::app_on("/", Rc::new(FailingSearchService));
    ui.search("serde");
    assert!(!ui.has("Results"));
    assert!(ui.text("Error").contains("crates.io is not reachable"), "{}", ui.text("Error"));
}

#[test]
fn a_result_opens_the_page_of_its_crate_and_back_returns_to_the_search() {
    let (ui, browser) = Ui::app();
    ui.search("serde");
    ui.click("Result-serde");
    assert!(ui.has("Details") && !ui.has("Search"));
    assert_eq!(ui.text("CrateName"), "serde");
    assert_eq!(ui.text("Version"), "Version 1.0.228");
    assert_eq!(ui.text("Downloads"), "612,480,315 downloads");
    assert!(ui.has("Repository") && ui.has("Documentation"));
    assert_eq!(browser.find("serde"), Some(catalogue_crate("serde")), "the browser keeps the crate it opened");

    ui.click("Back");
    assert!(!ui.has("Details"));
    assert_eq!(ui.text("Search"), "serde", "the search text outlives the page");
    ui.settle_until("the results", |ui| ui.has("Results"));
    assert_eq!(ui.results(), ["serde"]);
}

#[test]
fn the_page_of_a_crate_no_result_showed_is_replaced_by_its_error_and_tried_again() {
    let (ui, browser) = Ui::app_on("/crate/serde", Rc::new(CatalogueSearchService::new()));
    assert!(ui.has("Failure") && !ui.has("Details"), "the error boundary shows the error of the page");
    assert!(ui.text("ErrorMessage").contains("`serde`"), "{}", ui.text("ErrorMessage"));

    // Tried again with nothing changed, the page fails again.
    ui.click("TryAgain");
    assert!(ui.has("Failure"));

    // Tried again once the crate is known, the page shows it.
    browser.remember(catalogue_crate("serde"));
    ui.click("TryAgain");
    assert!(!ui.has("Failure"));
    assert_eq!(ui.text("CrateName"), "serde");

    // The browser started on this page: back is the search.
    ui.click("Back");
    assert!(ui.has("Search") && !ui.has("Details"));
}

#[test]
fn the_error_of_a_page_leads_back_to_the_search() {
    let (ui, _browser) = Ui::app_on("/crate/nothing", Rc::new(CatalogueSearchService::new()));
    assert!(ui.has("Failure"));
    ui.click("BackToSearch");
    assert!(ui.has("Search") && !ui.has("Failure"));
}
