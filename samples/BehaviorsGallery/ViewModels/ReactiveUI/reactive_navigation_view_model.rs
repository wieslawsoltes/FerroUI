//! Port of `ViewModels/ReactiveUI/ReactiveNavigationViewModel.cs` of the reference sample.

use super::detail_page_view_model::DetailPageViewModel;
use super::home_page_view_model::HomePageViewModel;
use crate::pages::detail_page_view::DetailPageView;
use crate::pages::home_page_view::HomePageView;
use crate::view_models::ViewModelBase;
use ferroui_base::ferro_markup_type;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{reactive_object, IScreen, RegisterViewForExt, RoutableViewModel, RoutingState};
use ferroui_splat::{Locator, MutableDependencyResolverExt};
use std::rc::{Rc, Weak};

/// The screen of the navigation page: its router, and the two pages it navigates to
/// (`ReactiveNavigationViewModel : ViewModelBase, IScreen`).
pub struct ReactiveNavigationViewModel {
    base: ViewModelBase,
    router: Rc<RoutingState>,
    home_page_view_model: Rc<HomePageViewModel>,
    details_view_model: Rc<DetailPageViewModel>,
}
reactive_object!(ReactiveNavigationViewModel, base);

impl PartialEq for ReactiveNavigationViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ReactiveNavigationViewModel {
    /// `ReactiveNavigationViewModel()`: registers the page view models and their views with the
    /// service locator, and navigates to the home page.
    pub fn new() -> Rc<Self> {
        let view_model = Rc::new_cyclic(|this: &Weak<Self>| {
            let screen: Weak<dyn IScreen> = this.clone();
            Self {
                base: ViewModelBase::new(),
                router: RoutingState::new(),
                home_page_view_model: HomePageViewModel::new(screen.clone()),
                details_view_model: DetailPageViewModel::new(screen),
            }
        });

        let resolver = Locator::current_mutable();
        let screen: Weak<dyn IScreen> = Rc::downgrade(&view_model) as Weak<dyn IScreen>;
        let home_screen = screen.clone();
        resolver.register::<HomePageViewModel>(move || Some(HomePageViewModel::new(home_screen.clone())));
        let detail_screen = screen;
        resolver.register::<DetailPageViewModel>(move || Some(DetailPageViewModel::new(detail_screen.clone())));

        resolver.register_view_for::<HomePageViewModel>(|| HomePageView::view_for_of(&HomePageView::new()), None);
        resolver.register_view_for::<DetailPageViewModel>(|| DetailPageView::view_for_of(&DetailPageView::new()), None);

        let home: RoutableViewModel = view_model.home_page_view_model.clone();
        let _ = view_model.router.navigate().execute(Some(home)).subscribe_ignore();
        view_model
    }

    /// `Router`.
    pub fn router(&self) -> Rc<RoutingState> {
        self.router.clone()
    }

    /// `HomePageViewModel`.
    pub fn home_page_view_model(&self) -> Rc<HomePageViewModel> {
        self.home_page_view_model.clone()
    }

    /// `DetailsViewModel`.
    pub fn details_view_model(&self) -> Rc<DetailPageViewModel> {
        self.details_view_model.clone()
    }
}

impl IScreen for ReactiveNavigationViewModel {
    fn router(&self) -> Rc<RoutingState> {
        self.router.clone()
    }
}

type Vm = Rc<ReactiveNavigationViewModel>;

ferro_markup_type!(class ReactiveNavigationViewModel {
    this: Rc<ReactiveNavigationViewModel>,
    handles: [ReactiveNavigationViewModel, Rc<ReactiveNavigationViewModel>, Option<Rc<ReactiveNavigationViewModel>>],
    constructors: [() => ReactiveNavigationViewModel::new],
    properties: [
        Router: Rc<RoutingState> { get: |this: &Vm| ReactiveNavigationViewModel::router(this) },
        HomePageViewModel: Rc<HomePageViewModel> { get: |this: &Vm| this.home_page_view_model() },
        DetailsViewModel: Rc<DetailPageViewModel> { get: |this: &Vm| this.details_view_model() },
    ],
    notify_property_changed: ReactiveNavigationViewModel,
});
