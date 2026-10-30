//! Port of `Views/ReactiveUI/DetailPageView` of the reference sample: the class of
//! `pages/DetailPageView.xaml`, the view the navigation page shows for its detail page.

use crate::markup::xaml_class;
use crate::view_models::reactive_ui::detail_page_view_model::DetailPageViewModel;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_reactiveui::IViewForOf;
use ferroui_reactiveui_controls::{ReactiveUserControl, ReactiveUserControlBase};
use std::rc::Rc;

/// The view of the detail page (`DetailPageView : ReactiveUserControl<DetailPageViewModel>`).
#[repr(C)]
pub struct DetailPageView {
    base: ReactiveUserControlBase,
}

::ferroui_base::ferro_class!(DetailPageView: ReactiveUserControlBase);
::ferroui_base::ferro_impl_classes!(
    DetailPageView: ::ferroui_base::FerroObjectImpl,
    ::ferroui_base::StyledElementImpl,
    ::ferroui_base::VisualImpl,
    ::ferroui_base::layout::LayoutableImpl,
    ::ferroui_base::interactivity::InteractiveImpl,
    ::ferroui_base::input::InputElementImpl,
    ::ferroui_controls::ControlImpl,
    ::ferroui_controls::primitives::TemplatedControlImpl,
    ::ferroui_controls::ContentControlImpl
);
ferro_class_info!(DetailPageView { new: DetailPageView::new });
xaml_class!(DetailPageView, "/pages/DetailPageView.xaml");

impl DetailPageView {
    pub fn construct() -> Self {
        Self {
            base: ReactiveUserControlBase::construct(),
        }
    }

    /// `DetailPageView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let _ = Self::typed(&this);
        this
    }

    /// The view typed by its view model (`ReactiveUserControl<DetailPageViewModel>`).
    pub fn typed(this: &Ref<Self>) -> ReactiveUserControl<DetailPageViewModel> {
        ReactiveUserControl::<DetailPageViewModel>::from_control(this.clone())
    }

    /// The view as `IViewFor<DetailPageViewModel>`.
    pub fn view_for_of(this: &Ref<Self>) -> Rc<dyn IViewForOf<DetailPageViewModel>> {
        Self::typed(this).into_view_for_of()
    }
}
