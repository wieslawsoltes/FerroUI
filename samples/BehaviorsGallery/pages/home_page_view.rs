//! Port of `Views/ReactiveUI/HomePageView` of the reference sample: the class of
//! `pages/HomePageView.xaml`, the view the navigation page shows for its home page.

use crate::markup::xaml_class;
use crate::view_models::reactive_ui::home_page_view_model::HomePageViewModel;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_reactiveui::IViewForOf;
use ferroui_reactiveui_controls::{ReactiveUserControl, ReactiveUserControlBase};
use std::rc::Rc;

/// The view of the home page (`HomePageView : ReactiveUserControl<HomePageViewModel>`).
#[repr(C)]
pub struct HomePageView {
    base: ReactiveUserControlBase,
}

::ferroui_base::ferro_class!(HomePageView: ReactiveUserControlBase);
::ferroui_base::ferro_impl_classes!(
    HomePageView: ::ferroui_base::FerroObjectImpl,
    ::ferroui_base::StyledElementImpl,
    ::ferroui_base::VisualImpl,
    ::ferroui_base::layout::LayoutableImpl,
    ::ferroui_base::interactivity::InteractiveImpl,
    ::ferroui_base::input::InputElementImpl,
    ::ferroui_controls::ControlImpl,
    ::ferroui_controls::primitives::TemplatedControlImpl,
    ::ferroui_controls::ContentControlImpl
);
ferro_class_info!(HomePageView { new: HomePageView::new });
xaml_class!(HomePageView, "/pages/HomePageView.xaml");

impl HomePageView {
    pub fn construct() -> Self {
        Self {
            base: ReactiveUserControlBase::construct(),
        }
    }

    /// `HomePageView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let _ = Self::typed(&this);
        this
    }

    /// The view typed by its view model (`ReactiveUserControl<HomePageViewModel>`).
    pub fn typed(this: &Ref<Self>) -> ReactiveUserControl<HomePageViewModel> {
        ReactiveUserControl::<HomePageViewModel>::from_control(this.clone())
    }

    /// The view as `IViewFor<HomePageViewModel>`.
    pub fn view_for_of(this: &Ref<Self>) -> Rc<dyn IViewForOf<HomePageViewModel>> {
        Self::typed(this).into_view_for_of()
    }
}
