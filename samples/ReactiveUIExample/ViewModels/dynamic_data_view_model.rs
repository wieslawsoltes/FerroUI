//! The view model of the DynamicData page: a source cache of the showcase's features, filtered by a
//! throttled search text, sorted, transformed into item view models and bound with
//! `sort_and_bind`, with `auto_refresh` on the favourite flag of the features.
//!
//! Addition of the port with no upstream counterpart (docs/porting/reactive.md, section 7.3): the
//! reference example has no DynamicData page, and no current ReactiveUI sample uses DynamicData. The
//! page follows the pattern of the one upstream sample that used DynamicData offline
//! (`ItemsDDViewModel` of the archived WinForms demo).

use super::page_view_model::page_view_model;
use super::{FeatureItemViewModel, PageViewModel};
use crate::models::ShowcaseFeature;
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand as FerroCommand;
use ferroui_dynamic_data::binding::{ObservableCollectionExtended, SortExpressionComparer};
use ferroui_dynamic_data::cache::{IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache, SourceCacheEx};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{IReactiveObjectExt, IScreen, ReactiveCommand, ReactiveUIBindingExtensions, RxSchedulers};
use ferroui_reactiveui_controls::ReactiveCommandAdapter;
use ferroui_system::{prop, IDisposable, IObservable};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// How long the search text has to stay unchanged before the list is filtered again.
pub const SEARCH_THROTTLE: Duration = Duration::from_millis(300);

/// The area of the features the page adds.
pub const ADDED_FEATURE_AREA: &str = "Added";

/// A predicate of the features the list shows.
type FeaturePredicate = Rc<dyn Fn(&Rc<ShowcaseFeature>) -> bool>;

/// The view model of the DynamicData page.
///
/// The features live in a [`SourceCache`] keyed by their id. The list is the pipeline
/// `connect()`, `auto_refresh(IsFavourite)`, `filter_dynamic(search)`, `transform(row)`,
/// `sort_and_bind(items, favourites first, then by name)`; the search text is observed with
/// `when_any_value` and throttled. `AddFeature` adds a feature with `add_or_update`, and
/// `RemoveFeature` removes one by its key; a row toggles the favourite flag of its feature, which
/// the auto refresh re-sorts.
pub struct DynamicDataViewModel {
    page: PageViewModel,
    /// A weak handle to the view model, for the handlers it registers.
    this: Weak<DynamicDataViewModel>,
    /// The features (`SourceCache<ShowcaseFeature, int>`).
    source: SourceCache<Rc<ShowcaseFeature>, i32>,
    /// The rows the list shows, sorted (`ObservableCollectionExtended<FeatureItemViewModel>`).
    items: ObservableCollectionExtended<Rc<FeatureItemViewModel>>,
    /// The search text (`SearchText`).
    search_text: RefCell<String>,
    /// The name of the feature `AddFeature` adds (`NewFeatureName`).
    new_feature_name: RefCell<String>,
    /// The key of the next feature `AddFeature` adds.
    next_id: Cell<i32>,
    /// The number of rows the list shows and the number of features (`Status`).
    status: RefCell<String>,
    /// Adds a feature named `NewFeatureName` (`AddFeature`).
    add_feature: OnceCell<ReactiveCommand<(), ()>>,
    /// Removes the feature of a key (`RemoveFeature`).
    remove_feature: OnceCell<ReactiveCommand<i32, ()>>,
    /// `AddFeature` as a command of the controls of FerroUI, for the bindings of markup.
    add_feature_command: OnceCell<Rc<dyn FerroCommand>>,
}
page_view_model!(DynamicDataViewModel);

impl DynamicDataViewModel {
    /// The page of `host_screen`, whose search is throttled on the scheduler of the user-interface
    /// thread.
    pub fn new(host_screen: &Rc<dyn IScreen>) -> Rc<Self> {
        Self::with_scheduler(host_screen, None)
    }

    /// The page of `host_screen`, whose search is throttled on `scheduler` (`None` for the scheduler
    /// of the user-interface thread); a test passes a virtual-time scheduler.
    pub fn with_scheduler(host_screen: &Rc<dyn IScreen>, scheduler: Option<Rc<dyn IScheduler>>) -> Rc<Self> {
        let scheduler = scheduler.unwrap_or_else(RxSchedulers::main_thread_scheduler);
        let source = SourceCache::new(|feature: &Rc<ShowcaseFeature>| feature.id());
        let features = ShowcaseFeature::defaults();
        let next_id = features.iter().map(|feature| feature.id()).max().unwrap_or(0) + 1;
        source.add_or_update_items(features);

        let view_model = Rc::new_cyclic(|this| Self {
            page: PageViewModel::new(
                host_screen,
                "dynamic-data",
                "DynamicData",
                "A SourceCache filtered by a throttled search, sorted with favourites first, transformed into rows and bound with SortAndBind.",
            ),
            this: this.clone(),
            source,
            items: ObservableCollectionExtended::new(),
            search_text: RefCell::new(String::new()),
            new_feature_name: RefCell::new(String::new()),
            next_id: Cell::new(next_id),
            status: RefCell::new(String::new()),
            add_feature: OnceCell::new(),
            remove_feature: OnceCell::new(),
            add_feature_command: OnceCell::new(),
        });
        let base = view_model.page.base();

        // RemoveFeature: `Remove(key)` on the source cache.
        let removed_from = view_model.source.clone();
        let remove_feature = base.track(ReactiveCommand::create_param(move |key: i32| removed_from.remove_key(key)));

        // AddFeature: `AddOrUpdate(feature)` on the source cache, while a name is entered.
        let can_add = view_model.when_any_value_select(prop!(DynamicDataViewModel => new_feature_name), |name: String| !name.trim().is_empty());
        let adding = view_model.this.clone();
        let add_feature = base.track(ReactiveCommand::create_with(
            move || {
                if let Some(view_model) = adding.upgrade() {
                    view_model.add_feature_now();
                }
            },
            Some(can_add),
            None,
        ));

        // The search: the current text at once, then each change once it has been unchanged for
        // the throttle.
        let initial_text = view_model.search_text();
        let predicates: Rc<dyn IObservable<FeaturePredicate>> = view_model
            .when_any_value(prop!(DynamicDataViewModel => search_text))
            .skip(1)
            .throttle_on(SEARCH_THROTTLE, scheduler)
            .start_with([initial_text])
            .distinct_until_changed()
            .select(|text: String| -> FeaturePredicate { Rc::new(move |feature: &Rc<ShowcaseFeature>| feature.matches(&text)) });

        // Favourites first, then by name.
        let comparer = SortExpressionComparer::<Rc<FeatureItemViewModel>>::descending(|row| row.is_favourite())
            .then_by_ascending(|row| row.name())
            .comparer();

        let row_remove = remove_feature.clone();
        let updated = view_model.this.clone();
        let subscription = view_model
            .source
            .connect()
            .auto_refresh(prop!(ShowcaseFeature => is_favourite))
            .filter_dynamic(predicates)
            .transform(move |feature: &Rc<ShowcaseFeature>| FeatureItemViewModel::new(feature.clone(), row_remove.clone()))
            .sort_and_bind(&view_model.items, comparer)
            .subscribe_next(move |_| {
                if let Some(view_model) = updated.upgrade() {
                    view_model.update_status();
                }
            });
        let _ = base.track_disposable(subscription);
        view_model.update_status();

        let _ = view_model.add_feature_command.set(ReactiveCommandAdapter::command_of(Rc::new(add_feature.clone())));
        let _ = view_model.add_feature.set(add_feature);
        let _ = view_model.remove_feature.set(remove_feature);
        view_model
    }

    /// The features of the source cache (`Source`).
    pub fn source(&self) -> SourceCache<Rc<ShowcaseFeature>, i32> {
        self.source.clone()
    }

    /// The rows the list shows, sorted (`Items`).
    pub fn items(&self) -> ObservableCollectionExtended<Rc<FeatureItemViewModel>> {
        self.items.clone()
    }

    /// The search text (`SearchText { get; }`).
    pub fn search_text(&self) -> String {
        self.search_text.borrow().clone()
    }

    /// Sets the search text (`SearchText { set; }`); the list is filtered again once the text has
    /// been unchanged for [`SEARCH_THROTTLE`].
    pub fn set_search_text(&self, value: String) {
        let _ = self.raise_and_set_if_changed(&self.search_text, value, "SearchText");
    }

    /// The name of the feature `AddFeature` adds (`NewFeatureName { get; }`).
    pub fn new_feature_name(&self) -> String {
        self.new_feature_name.borrow().clone()
    }

    /// Sets the name of the feature `AddFeature` adds (`NewFeatureName { set; }`).
    pub fn set_new_feature_name(&self, value: String) {
        let _ = self.raise_and_set_if_changed(&self.new_feature_name, value, "NewFeatureName");
    }

    /// The number of rows the list shows and the number of features (`Status`).
    pub fn status(&self) -> String {
        self.status.borrow().clone()
    }

    /// The command that adds a feature named `NewFeatureName` (`AddFeature`).
    pub fn add_feature(&self) -> ReactiveCommand<(), ()> {
        self.add_feature.get().expect("the command is created with the view model").clone()
    }

    /// The command that removes the feature of a key (`RemoveFeature`).
    pub fn remove_feature(&self) -> ReactiveCommand<i32, ()> {
        self.remove_feature.get().expect("the command is created with the view model").clone()
    }

    /// Adds a feature named `NewFeatureName` to the source cache with `add_or_update`, under the next
    /// key, and clears the name.
    fn add_feature_now(&self) {
        let name = self.new_feature_name().trim().to_owned();
        if name.is_empty() {
            return;
        }
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.source.add_or_update(ShowcaseFeature::new(id, name, ADDED_FEATURE_AREA));
        self.set_new_feature_name(String::new());
    }

    /// Updates the status after a change of the list.
    fn update_status(&self) {
        let status = format!("Showing {} of {} features.", self.items.count(), self.source.count());
        let _ = self.raise_and_set_if_changed(&self.status, status, "Status");
    }
}

impl IDisposable for DynamicDataViewModel {
    /// Disposes the pipeline and the commands, then the source cache.
    fn dispose(&self) {
        if self.page.base().is_disposed() {
            return;
        }
        self.page.base().dispose();
        self.source.dispose();
    }
}

ferro_markup_type!(class DynamicDataViewModel {
    this: Rc<DynamicDataViewModel>,
    handles: [DynamicDataViewModel, Rc<DynamicDataViewModel>, Option<Rc<DynamicDataViewModel>>],
    properties: [
        Title: String { get: |this: &Rc<DynamicDataViewModel>| this.title() },
        Summary: String { get: |this: &Rc<DynamicDataViewModel>| this.summary() },
        Status: String { get: |this: &Rc<DynamicDataViewModel>| this.status() },
        SearchText: String {
            get: |this: &Rc<DynamicDataViewModel>| this.search_text(),
            set: |this: &Rc<DynamicDataViewModel>, value: String| this.set_search_text(value)
        },
        NewFeatureName: String {
            get: |this: &Rc<DynamicDataViewModel>| this.new_feature_name(),
            set: |this: &Rc<DynamicDataViewModel>, value: String| this.set_new_feature_name(value)
        },
        AddFeature: Rc<dyn FerroCommand> {
            get: |this: &Rc<DynamicDataViewModel>| this.add_feature_command.get().expect("the command is created with the view model").clone()
        },
    ],
    notify_property_changed: DynamicDataViewModel,
});
