//! Port of `ViewModels/ReactiveUI/HomePageViewModel.cs` of the reference sample.

use crate::view_models::ViewModelBase;
use ferroui_base::ferro_markup_type;
use ferroui_reactiveui::{reactive_object, IRoutableViewModel, IScreen};
use std::rc::{Rc, Weak};

/// The home page of the navigation page (`HomePageViewModel(IScreen) : ViewModelBase, IRoutableViewModel`).
pub struct HomePageViewModel {
    base: ViewModelBase,
    /// `HostScreen`, held weakly: the screen owns the router, whose stack holds this view model.
    host_screen: Weak<dyn IScreen>,
}
reactive_object!(HomePageViewModel, base);

impl PartialEq for HomePageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl HomePageViewModel {
    /// `HomePageViewModel(IScreen screen)`.
    pub fn new(screen: Weak<dyn IScreen>) -> Rc<Self> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            host_screen: screen,
        })
    }
}

impl IRoutableViewModel for HomePageViewModel {
    /// `UrlPathSegment`.
    fn url_path_segment(&self) -> Option<String> {
        Some("home".to_string())
    }

    /// `HostScreen`.
    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.host_screen.upgrade()
    }
}

ferro_markup_type!(class HomePageViewModel {
    this: Rc<HomePageViewModel>,
    handles: [HomePageViewModel, Rc<HomePageViewModel>, Option<Rc<HomePageViewModel>>],
    properties: [
        UrlPathSegment: Option<String> { get: |this: &Rc<HomePageViewModel>| this.url_path_segment() },
    ],
    notify_property_changed: HomePageViewModel,
});
