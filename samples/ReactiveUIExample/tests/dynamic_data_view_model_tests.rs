//! The tests of the view model of the DynamicData page: the filtering by the throttled search text,
//! the sorting, the additions and removals of the commands, and the re-sorting of the auto refresh.
//!
//! Not a port: the DynamicData page is an addition of the port with no upstream counterpart
//! (docs/porting/reactive.md, section 7.3). The throttle runs on a virtual-time scheduler.

use super::support::{run_jobs, start_application, Disposing};
use crate::models::ShowcaseFeature;
use crate::view_models::{DynamicDataViewModel, FeatureItemViewModel, ADDED_FEATURE_AREA, SEARCH_THROTTLE};
use ferroui_dynamic_data::cache::{IObservableCache, SourceCacheEx};
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{duration_to_ticks, TestScheduler};
use ferroui_reactiveui::{reactive_object, ICommand, IScreen, ReactiveObject, RoutingState};
use std::rc::Rc;

/// A screen for the page (`IScreen`).
struct TestScreen {
    reactive: ReactiveObject,
    router: Rc<RoutingState>,
}
reactive_object!(TestScreen);

impl IScreen for TestScreen {
    fn router(&self) -> Rc<RoutingState> {
        self.router.clone()
    }
}

/// The page under test, with its search throttled on `scheduler`, and its screen.
struct Page {
    view_model: Disposing<DynamicDataViewModel>,
    scheduler: TestScheduler,
    _screen: Rc<dyn IScreen>,
}

impl Page {
    fn new() -> Self {
        let screen: Rc<dyn IScreen> = Rc::new(TestScreen { reactive: ReactiveObject::new(), router: RoutingState::new() });
        let scheduler = TestScheduler::new();
        let view_model = DynamicDataViewModel::with_scheduler(&screen, Some(scheduler.scheduler()));
        Self { view_model: Disposing(view_model), scheduler, _screen: screen }
    }

    fn view_model(&self) -> &Rc<DynamicDataViewModel> {
        &self.view_model.0
    }

    /// The rows of the list, in their order.
    fn rows(&self) -> Vec<Rc<FeatureItemViewModel>> {
        self.view_model().items().to_vec()
    }

    /// The names of the rows of the list, in their order.
    fn names(&self) -> Vec<String> {
        self.rows().iter().map(|row| row.name()).collect()
    }

    /// Lets the throttle of the search pass.
    fn pass_throttle(&self) {
        self.scheduler.advance_by(duration_to_ticks(SEARCH_THROTTLE) + 1);
    }

    /// Enters `text` as the search and lets the throttle pass.
    fn search(&self, text: &str) {
        self.view_model().set_search_text(text.to_owned());
        self.pass_throttle();
    }
}

/// The names of the default features, sorted by name.
fn default_names_sorted() -> Vec<String> {
    let mut names: Vec<String> = ShowcaseFeature::defaults().iter().map(|feature| feature.name()).collect();
    names.sort();
    names
}

#[test]
fn initial_rows_show_every_feature_sorted_by_name() {
    let _app = start_application();
    let page = Page::new();

    assert_eq!(page.names(), default_names_sorted());
    assert_eq!(page.view_model().status(), format!("Showing {0} of {0} features.", ShowcaseFeature::defaults().len()));
}

#[test]
fn search_filters_the_rows_once_the_throttle_passes() {
    let _app = start_application();
    let page = Page::new();
    let all = page.names();

    page.view_model().set_search_text("ROUTING".to_owned());
    assert_eq!(page.names(), all, "the search waits for the throttle");

    page.pass_throttle();
    let rows = page.rows();
    assert!(!rows.is_empty());
    assert!(rows.iter().all(|row| row.feature().matches("routing")));
    assert!(rows.len() < all.len());
    assert_eq!(page.view_model().status(), format!("Showing {} of {} features.", rows.len(), all.len()));

    page.search("");
    assert_eq!(page.names(), all);
}

#[test]
fn search_keeps_only_the_last_text_of_a_burst() {
    let _app = start_application();
    let page = Page::new();
    let all = page.names();

    page.view_model().set_search_text("Rout".to_owned());
    page.scheduler.advance_by(duration_to_ticks(SEARCH_THROTTLE) / 2);
    page.view_model().set_search_text("Collections".to_owned());
    page.scheduler.advance_by(duration_to_ticks(SEARCH_THROTTLE) / 2);
    assert_eq!(page.names(), all, "a change within the throttle restarts it");

    page.pass_throttle();
    assert_eq!(page.names(), vec!["DynamicData SourceCache with SortAndBind".to_owned()]);
}

#[test]
fn add_feature_adds_with_add_or_update_at_its_sorted_position() {
    let _app = start_application();
    let page = Page::new();
    let view_model = page.view_model();
    let count = view_model.source().count();
    run_jobs();
    assert!(!ICommand::can_execute(&view_model.add_feature(), None), "no name is entered");

    view_model.set_new_feature_name("Commands bound with BindCommand".to_owned());
    run_jobs();
    assert!(ICommand::can_execute(&view_model.add_feature(), None));
    let _ = view_model.add_feature().execute(()).subscribe_next(|_| {});
    run_jobs();

    assert_eq!(view_model.source().count(), count + 1);
    let mut expected = default_names_sorted();
    expected.push("Commands bound with BindCommand".to_owned());
    expected.sort();
    assert_eq!(page.names(), expected);
    let added = page.rows().into_iter().find(|row| row.name() == "Commands bound with BindCommand").expect("the added row");
    assert_eq!(added.area(), ADDED_FEATURE_AREA);
    assert_eq!(view_model.new_feature_name(), "", "the name is cleared");
}

#[test]
fn added_feature_is_filtered_by_the_current_search() {
    let _app = start_application();
    let page = Page::new();
    let view_model = page.view_model();
    page.search("Routing");
    let filtered = page.names();

    view_model.set_new_feature_name("Animation timeline".to_owned());
    let _ = view_model.add_feature().execute(()).subscribe_next(|_| {});
    run_jobs();

    assert_eq!(page.names(), filtered, "a feature the search does not match is not shown");
    page.search("");
    assert!(page.names().contains(&"Animation timeline".to_owned()));
}

#[test]
fn remove_feature_removes_the_row_by_its_key() {
    let _app = start_application();
    let page = Page::new();
    let view_model = page.view_model();
    let first = page.rows()[0].clone();
    let count = view_model.source().count();

    let _ = view_model.remove_feature().execute(first.id()).subscribe_next(|_| {});
    run_jobs();

    assert_eq!(view_model.source().count(), count - 1);
    assert!(view_model.source().lookup(&first.id()).is_none());
    assert!(!page.names().contains(&first.name()));
}

#[test]
fn row_remove_command_removes_its_feature() {
    let _app = start_application();
    let page = Page::new();
    let row = page.rows()[2].clone();
    let count = page.view_model().source().count();

    let _ = row.remove().execute(()).subscribe_next(|_| {});
    run_jobs();

    assert_eq!(page.view_model().source().count(), count - 1);
    assert!(!page.names().contains(&row.name()));
}

#[test]
fn favourite_toggle_resorts_the_rows_through_auto_refresh() {
    let _app = start_application();
    let page = Page::new();
    let names = page.names();
    let last = page.rows().last().cloned().expect("a last row");

    last.feature().set_is_favourite(true);
    run_jobs();
    let favourite_first = page.names();
    assert_eq!(favourite_first[0], last.name(), "the favourite moves to the top");
    assert_eq!(&favourite_first[1..], &names[..names.len() - 1]);

    let _ = last.toggle_favourite().execute(()).subscribe_next(|_| {});
    run_jobs();
    assert!(!last.is_favourite());
    assert_eq!(page.names(), names, "the row goes back to its place");
}

#[test]
fn favourites_sort_by_name_among_themselves() {
    let _app = start_application();
    let page = Page::new();
    let rows = page.rows();
    let (second, fifth) = (rows[1].clone(), rows[4].clone());

    fifth.feature().set_is_favourite(true);
    second.feature().set_is_favourite(true);
    run_jobs();

    let names = page.names();
    assert_eq!(names[0], second.name());
    assert_eq!(names[1], fifth.name());
}

#[test]
fn source_edits_reach_the_rows() {
    let _app = start_application();
    let page = Page::new();
    let source = page.view_model().source();

    source.add_or_update(ShowcaseFeature::new(100, "A feature added to the cache", "Collections"));
    run_jobs();
    assert_eq!(page.names()[0], "A feature added to the cache");

    source.remove_key(100);
    run_jobs();
    assert_eq!(page.names(), default_names_sorted());
}
