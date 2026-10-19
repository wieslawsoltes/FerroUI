//! Port of `ViewModels/OverviewViewModel.cs`: introduces the showcase features.

use super::page_view_model::page_view_model;
use super::{FeatureViewModel, PageViewModel};
use ferroui_base::ferro_markup_type;
use ferroui_reactiveui::IScreen;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// Introduces the showcase features (`OverviewViewModel`).
pub struct OverviewViewModel {
    page: PageViewModel,
    /// The feature labels rendered by the overview page (`Features`).
    features: Vec<Rc<FeatureViewModel>>,
}
page_view_model!(OverviewViewModel);

impl OverviewViewModel {
    /// The overview of `host_screen` (`OverviewViewModel(IScreen)`).
    pub fn new(host_screen: &Rc<dyn IScreen>) -> Rc<Self> {
        Rc::new(Self {
            page: PageViewModel::new(
                host_screen,
                "overview",
                // Deviation (DEVIATIONS.md, samples): the names of the reference's product are the
                // names of the port's (`ReactiveUI FerroUI`).
                "ReactiveUI FerroUI showcase",
                "A compact tour through routing, binding, activation, interactions, and live local data.",
            ),
            features: vec![
                FeatureViewModel::new("RoutedViewHost shell navigation"),
                FeatureViewModel::new("ViewModelViewHost contracts and automatic item templates"),
                FeatureViewModel::new("ReactiveUI Bind, OneWayBind, and FerroUI compiled bindings"),
                FeatureViewModel::new("ReactiveCommand can-execute, async execution, and failures"),
                FeatureViewModel::new("ObservableAsPropertyHelper computed state"),
                FeatureViewModel::new("WhenActivated disposal for live subscriptions"),
                FeatureViewModel::new("GetSubject and GetBindingSubject property bridges"),
                FeatureViewModel::new("AutoSuspendHelper restores input and threshold between sessions"),
                // An addition of the port: the DynamicData page has no counterpart in the reference.
                FeatureViewModel::new("DynamicData SourceCache filtered, sorted and bound with SortAndBind"),
            ],
        })
    }

    /// The feature labels rendered by the overview page (`Features`).
    pub fn features(&self) -> Vec<Rc<FeatureViewModel>> {
        self.features.clone()
    }
}

impl IDisposable for OverviewViewModel {
    /// `Dispose()`.
    fn dispose(&self) {
        self.page.base().dispose();
    }
}

ferro_markup_type!(class OverviewViewModel {
    this: Rc<OverviewViewModel>,
    handles: [OverviewViewModel, Rc<OverviewViewModel>, Option<Rc<OverviewViewModel>>],
    properties: [
        Title: String { get: |this: &Rc<OverviewViewModel>| this.title() },
        Summary: String { get: |this: &Rc<OverviewViewModel>| this.summary() },
    ],
    notify_property_changed: OverviewViewModel,
});
