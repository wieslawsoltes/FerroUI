//! Port of `ViewModels/ReactiveUI/DetailPageViewModel.cs` of the reference sample.

use crate::view_models::ViewModelBase;
use ferroui_base::ferro_markup_type;
use ferroui_reactiveui::{reactive_object, IRoutableViewModel, IScreen};
use std::rc::{Rc, Weak};

/// The detail page of the navigation page (`DetailPageViewModel(IScreen) : ViewModelBase, IRoutableViewModel`).
pub struct DetailPageViewModel {
    base: ViewModelBase,
    /// `HostScreen`, held weakly: the screen owns the router, whose stack holds this view model.
    host_screen: Weak<dyn IScreen>,
}
reactive_object!(DetailPageViewModel, base);

impl PartialEq for DetailPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl DetailPageViewModel {
    /// `DetailPageViewModel(IScreen screen)`.
    pub fn new(screen: Weak<dyn IScreen>) -> Rc<Self> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            host_screen: screen,
        })
    }
}

impl IRoutableViewModel for DetailPageViewModel {
    /// `UrlPathSegment`.
    fn url_path_segment(&self) -> Option<String> {
        Some("detail".to_string())
    }

    /// `HostScreen`.
    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        self.host_screen.upgrade()
    }
}

ferro_markup_type!(class DetailPageViewModel {
    this: Rc<DetailPageViewModel>,
    handles: [DetailPageViewModel, Rc<DetailPageViewModel>, Option<Rc<DetailPageViewModel>>],
    properties: [
        UrlPathSegment: Option<String> { get: |this: &Rc<DetailPageViewModel>| this.url_path_segment() },
    ],
    notify_property_changed: DetailPageViewModel,
});
