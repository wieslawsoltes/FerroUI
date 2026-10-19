//! Port of `Views/PerformanceMetricCardView.xaml.cs`: the class of the document `PerformanceMetricCardView.xaml`, a contracted metric card view for performance metrics (contract `performance`).

use crate::markup::{reactive_user_control_class, xaml_class};
use crate::view_models::MetricCardViewModel;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_reactiveui::IViewForOf;
use ferroui_reactiveui_controls::{ReactiveUserControl, ReactiveUserControlBase};
use std::rc::Rc;

/// A contracted metric card view for performance metrics (contract `performance`) (`PerformanceMetricCardView : ReactiveUserControl<MetricCardViewModel>`).
#[repr(C)]
pub struct PerformanceMetricCardView {
    base: ReactiveUserControlBase,
}

reactive_user_control_class!(PerformanceMetricCardView);
ferro_class_info!(PerformanceMetricCardView { new: PerformanceMetricCardView::new });
xaml_class!(PerformanceMetricCardView, "/Views/PerformanceMetricCardView.xaml");

impl PerformanceMetricCardView {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: ReactiveUserControlBase::construct() }
    }

    /// Creates the view (`PerformanceMetricCardView()`).
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let _ = Self::typed(&this);
        this
    }

    /// The view typed by its view model (`ReactiveUserControl<MetricCardViewModel>`).
    pub fn typed(this: &Ref<Self>) -> ReactiveUserControl<MetricCardViewModel> {
        ReactiveUserControl::<MetricCardViewModel>::from_control(this.clone())
    }

    /// The view as `IViewFor<MetricCardViewModel>`.
    pub fn view_for_of(this: &Ref<Self>) -> Rc<dyn IViewForOf<MetricCardViewModel>> {
        Self::typed(this).into_view_for_of()
    }

    /// The view model of the view (`ViewModel`).
    pub fn view_model(this: &Ref<Self>) -> Option<Rc<MetricCardViewModel>> {
        Self::typed(this).view_model()
    }
}
