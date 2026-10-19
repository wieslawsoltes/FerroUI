//! Not a port (the reference has no tests): the search of [`AppViewModel`] on the in-memory
//! catalogue, in the virtual time of a test scheduler that stands for both schedulers of
//! ReactiveUI (`TestScheduler.With`).

use super::support::{RecordingLauncher, RecordingSearchService};
use crate::nuget_search::CatalogueSearchService;
use crate::AppViewModel;
use ferroui_reactive_testing::TestScheduler;
use ferroui_reactiveui_testing::{SchedulerExtensions, TestSchedulerExtensions};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::cell::RefCell;
use std::rc::Rc;

/// The view model over `search`.
fn view_model(search: &Rc<RecordingSearchService>) -> Rc<AppViewModel> {
    AppViewModel::with_services(search.clone(), RecordingLauncher::source(&RecordingLauncher::new()))
}

/// The titles of the results of the view model, or `None` before the first search.
fn result_titles(view_model: &AppViewModel) -> Option<Vec<String>> {
    view_model
        .search_results()
        .map(|results| results.iter().map(|result| result.title().unwrap_or_default()).collect())
}

#[test]
fn the_search_waits_until_the_term_has_not_changed_for_800_ms() {
    TestScheduler::new().with(|scheduler| {
        let search = RecordingSearchService::new();
        let view_model = view_model(&search);

        view_model.set_search_term(Some("reactive".to_owned()));
        scheduler.advance_by_ms(500.0);
        assert!(search.terms().is_empty(), "no search while the term is being typed");

        // Each change starts the 800 ms again.
        view_model.set_search_term(Some("reactiveui".to_owned()));
        scheduler.advance_by_ms(799.0);
        assert!(search.terms().is_empty(), "no search before 800 ms without a change");

        scheduler.advance_by_ms(1.0);
        assert_eq!(search.terms(), ["reactiveui"], "one search, of the last term");
    });
}

#[test]
fn a_term_equal_to_the_previous_one_once_trimmed_is_not_searched_again() {
    TestScheduler::new().with(|scheduler| {
        let search = RecordingSearchService::new();
        let view_model = view_model(&search);

        view_model.set_search_term(Some("splat".to_owned()));
        scheduler.advance_by_ms(800.0);
        view_model.set_search_term(Some("  splat ".to_owned()));
        scheduler.advance_by_ms(800.0);
        assert_eq!(search.terms(), ["splat"], "the trimmed term is the same: DistinctUntilChanged drops it");

        view_model.set_search_term(Some("dynamic data".to_owned()));
        scheduler.advance_by_ms(800.0);
        view_model.set_search_term(Some("splat".to_owned()));
        scheduler.advance_by_ms(800.0);
        assert_eq!(search.terms(), ["splat", "dynamic data", "splat"], "a term differs from the previous one again");
    });
}

#[test]
fn an_empty_term_is_not_searched() {
    TestScheduler::new().with(|scheduler| {
        let search = RecordingSearchService::new();
        let view_model = view_model(&search);

        scheduler.advance_by_ms(1000.0);
        view_model.set_search_term(Some(String::new()));
        scheduler.advance_by_ms(1000.0);
        view_model.set_search_term(Some("   ".to_owned()));
        scheduler.advance_by_ms(1000.0);
        view_model.set_search_term(None);
        scheduler.advance_by_ms(1000.0);

        assert!(search.terms().is_empty(), "{:?}", search.terms());
        assert_eq!(result_titles(&view_model), None);
        assert!(!view_model.is_available());
    });
}

#[test]
fn the_results_are_the_packages_the_search_found() {
    TestScheduler::new().with(|scheduler| {
        let search = RecordingSearchService::new();
        let view_model = view_model(&search);
        assert_eq!(result_titles(&view_model), None, "no results before the first search");

        view_model.set_search_term(Some(" reactive testing ".to_owned()));
        scheduler.advance_by_ms(801.0);

        assert_eq!(search.terms(), ["reactive testing"], "the term is trimmed");
        let expected: Vec<String> = CatalogueSearchService::find("reactive testing").iter().map(|package| package.title()).collect();
        assert_eq!(expected, ["ReactiveUI.Testing", "Microsoft.Reactive.Testing"]);
        assert_eq!(result_titles(&view_model), Some(expected));

        // A later search replaces the results.
        view_model.set_search_term(Some("json".to_owned()));
        scheduler.advance_by_ms(801.0);
        assert_eq!(result_titles(&view_model), Some(vec!["Json.NET".to_owned()]));
    });
}

#[test]
fn the_results_are_delivered_on_the_main_thread_scheduler() {
    TestScheduler::new().with(|scheduler| {
        let search = RecordingSearchService::new();
        let view_model = view_model(&search);
        let changed = Rc::new(RefCell::new(Vec::<String>::new()));
        let log = changed.clone();
        view_model.property_changed().add(Rc::new(move |name: &str| log.borrow_mut().push(name.to_owned())));

        view_model.set_search_term(Some("akavache".to_owned()));
        scheduler.advance_by_ms(801.0);

        // `IsAvailable` follows `SearchResults` through a subscription the view model made before
        // this handler was added, so its change is raised inside that of `SearchResults`, before
        // this handler hears of `SearchResults` (handlers run in the order they were added).
        assert_eq!(*changed.borrow(), ["SearchTerm", "IsAvailable", "SearchResults"]);
        assert_eq!(result_titles(&view_model), Some(vec!["Akavache".to_owned()]));
    });
}

#[test]
fn is_available_follows_the_results() {
    TestScheduler::new().with(|scheduler| {
        let search = RecordingSearchService::new();
        let view_model = view_model(&search);
        assert!(!view_model.is_available(), "no results before the first search");

        // A search without a match still gives results, an empty list.
        view_model.set_search_term(Some("no such package".to_owned()));
        scheduler.advance_by_ms(801.0);
        assert_eq!(result_titles(&view_model), Some(Vec::new()));
        assert!(view_model.is_available());

        view_model.set_search_term(Some("polly".to_owned()));
        scheduler.advance_by_ms(801.0);
        assert_eq!(result_titles(&view_model), Some(vec!["Polly".to_owned()]));
        assert!(view_model.is_available());
    });
}
