//! Port of `AppViewModel.cs`.

use crate::crate_details_view_model::{CrateDetailsViewModel, LauncherSource};
use crate::crate_search::{default_search_service, ICrateSearchService};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{
    reactive_object, reactive_properties, ObservableAsPropertyHelper, ReactiveObject, ReactiveUIBindingExtensions, RxSchedulers,
    ToPropertyMixins,
};
use ferroui_system::threading::CancellationToken;
use ferroui_system::{prop, ObservableError};
use std::cell::{OnceCell, RefCell};
use std::future::Future;
use std::rc::Rc;
use std::time::Duration;

/// The results of a search (`IEnumerable<NugetDetailsViewModel>` of the reference, for crates):
/// `None` before the first search, as the reference's property is `null` then.
pub type SearchResults = Option<Rc<Vec<Rc<CrateDetailsViewModel>>>>;

reactive_properties! {
    /// AppViewModel is where we will describe the interaction of our application. We can describe
    /// the entire application in one class since it's very small now. Most ViewModels will derive
    /// off ReactiveObject, while most Model classes will derive off INotifyPropertyChanged.
    pub struct AppViewModel {
        reactive: ReactiveObject,
        // `#[reactive]` comes before the other attributes of its field (`reactive_properties!`).
        #[reactive]
        /// In ReactiveUI, this is the syntax to declare a read-write property that will notify
        /// Observers, as well as the views, that a property has changed. If we declared this as a
        /// normal property, we couldn't tell when it has changed!
        search_term: Option<String>,
        /// Here's the interesting part: In ReactiveUI, we can take IObservables and "pipe" them to
        /// a Property - whenever the Observable yields a new value, we will notify ReactiveObject
        /// that the property has changed.
        ///
        /// To do this, we have a class called ObservableAsPropertyHelper - this class subscribes to
        /// an Observable and stores a copy of the latest value. It also runs an action whenever the
        /// property changes, usually calling ReactiveObject's RaisePropertyChanged.
        search_results: OnceCell<ObservableAsPropertyHelper<SearchResults>>,
        /// Here, we want to create a property to represent when the application is performing a
        /// search (i.e. when to show the "spinner" control that lets the user know that the app is
        /// busy). We also declare this property to be the result of an Observable (i.e. its value
        /// is derived from some other property).
        is_available: OnceCell<ObservableAsPropertyHelper<bool>>,
    }
}
reactive_object!(AppViewModel);

impl PartialEq for AppViewModel {
    /// A view model is a reference type: two are equal when they are the same object.
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl AppViewModel {
    /// The view model of the application, which searches with the search service of the sample
    /// (`new AppViewModel()`) and opens pages with the launcher `launcher` gives.
    pub fn new(launcher: LauncherSource) -> Rc<Self> {
        Self::with_services(default_search_service(), launcher)
    }

    /// The view model, searching with `search_service`.
    ///
    /// Deviation (DEVIATIONS.md, samples): the reference searches in the view model itself and has
    /// no parameter; the service and the launcher are parameters, so that the tests give their own.
    pub fn with_services(search_service: Rc<dyn ICrateSearchService>, launcher: LauncherSource) -> Rc<Self> {
        let view_model = Rc::new(Self {
            reactive: ReactiveObject::new(),
            search_term: RefCell::new(None),
            search_results: OnceCell::new(),
            is_available: OnceCell::new(),
        });

        // Creating our UI declaratively
        //
        // The Properties in this ViewModel are related to each other in different ways - with
        // other frameworks, it is difficult to describe each relation succinctly; the code to
        // implement "The UI spinner spins while the search is live" usually ends up spread out over
        // several event handlers.
        //
        // However, with ReactiveUI, we can describe how properties are related in a very organized
        // clear way. Let's describe the workflow of what the user does in this application, in the
        // order they do it.

        // We're going to take a Property and turn it into an Observable here - this Observable will
        // yield a value every time the Search term changes, which in the markup, is connected to
        // the TextBox.
        //
        // We're going to use the Throttle operator to ignore changes that happen too quickly, since
        // we don't want to issue a search for each key pressed! We then pull the Value of the
        // change, then filter out changes that are identical, as well as strings that are empty.
        //
        // We then do a SelectMany() which starts the task by converting the task of the search into
        // an observable of its result. If subsequent requests are made, the CancellationToken is
        // called. We then ObserveOn the main thread, everything up until this point has been
        // running on a separate thread due to the Throttle().
        //
        // We then use a ObservableAsPropertyHelper and the ToProperty() method to allow us to have
        // the latest results that we can expose through the property to the View.
        let search_results = view_model
            .when_any_value(prop!(AppViewModel => search_term))
            // Deviation (DEVIATIONS.md, samples): upstream's Throttle waits on the default
            // scheduler of the thread pool. The observables of the port are single-threaded, and
            // the default scheduler of a thread waits by blocking it, which would block the
            // user-interface thread; the throttle waits on the main thread scheduler (the timers of
            // the dispatcher), and the search itself runs on a worker thread.
            .throttle_on(Duration::from_millis(800), RxSchedulers::main_thread_scheduler())
            .select(|term: Option<String>| term.map(|term| term.trim().to_owned()))
            .distinct_until_changed()
            .where_(|term: &Option<String>| term.as_deref().is_some_and(|term| !term.is_empty()))
            .select_many_async_with_token(move |term: Option<String>, token: CancellationToken| {
                search_crates(&search_service, &launcher, term.unwrap_or_default(), token)
            })
            .observe_on(RxSchedulers::main_thread_scheduler())
            .to_property(&view_model, prop!(AppViewModel => search_results));

        // We subscribe to the "ThrownExceptions" property of our OAPH, where ReactiveUI marshals any
        // exceptions that are thrown in the search_crates function. See the "Error Handling"
        // section for more information about this.
        let _ = search_results.thrown_exceptions().subscribe_next(|_error: ObservableError| { /* Handle errors here */ });
        let _ = view_model.search_results.set(search_results);

        // A helper method we can use for Visibility or Spinners to show if results are available.
        // We get the latest value of the SearchResults and make sure it's not null.
        let is_available = view_model
            .when_any_value(prop!(AppViewModel => search_results))
            .select(|search_results: SearchResults| search_results.is_some())
            .to_property(&view_model, prop!(AppViewModel => is_available));
        let _ = view_model.is_available.set(is_available);

        view_model
    }

    /// `SearchResults`: the view models of the crates of the latest search.
    pub fn search_results(&self) -> SearchResults {
        self.search_results.get().and_then(|helper| helper.value())
    }

    /// `IsAvailable`: whether there are results.
    pub fn is_available(&self) -> bool {
        self.is_available.get().is_some_and(|helper| helper.value())
    }
}

/// Here we search crates with the search service of the sample (`SearchNuGetPackages` of the
/// reference, which searches NuGet packages). Ideally, we should extract such code into a separate
/// service, say, ICrateSearchService, but let's try to avoid overcomplicating things at this time.
///
/// Deviation (DEVIATIONS.md, samples): the search itself is the service ([`ICrateSearchService`]),
/// which the reference suggests; the view models of the crates are made on the thread of the view
/// model when the search ends, as they are not `Send`. The reference searches NuGet; the port
/// searches crates.io, because FerroUI is a Rust framework.
fn search_crates(
    search_service: &Rc<dyn ICrateSearchService>,
    launcher: &LauncherSource,
    term: String,
    token: CancellationToken,
) -> impl Future<Output = Result<SearchResults, ObservableError>> + 'static {
    let metadata = search_service.search(&term, token);
    let launcher = launcher.clone();
    async move {
        let metadata = metadata.await?;
        let results: Vec<Rc<CrateDetailsViewModel>> =
            metadata.into_iter().map(|krate| CrateDetailsViewModel::new(krate, launcher.clone())).collect();
        Ok::<SearchResults, ObservableError>(Some(Rc::new(results)))
    }
}
