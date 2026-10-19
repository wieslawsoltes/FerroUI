//! Port of `Views/FeatureView.xaml.cs`: the class of the document `FeatureView.xaml`, a reusable feature row resolved by the automatic data-template binding hook.

use crate::markup::{reactive_user_control_class, xaml_class};
use crate::view_models::FeatureViewModel;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_reactiveui::IViewForOf;
use ferroui_reactiveui_controls::{ReactiveUserControl, ReactiveUserControlBase};
use std::rc::Rc;

/// A reusable feature row resolved by the automatic data-template binding hook (`FeatureView : ReactiveUserControl<FeatureViewModel>`).
#[repr(C)]
pub struct FeatureView {
    base: ReactiveUserControlBase,
}

reactive_user_control_class!(FeatureView);
ferro_class_info!(FeatureView { new: FeatureView::new });
xaml_class!(FeatureView, "/Views/FeatureView.xaml");

impl FeatureView {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: ReactiveUserControlBase::construct() }
    }

    /// Creates the view (`FeatureView()`).
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let _ = Self::typed(&this);
        this
    }

    /// The view typed by its view model (`ReactiveUserControl<FeatureViewModel>`).
    pub fn typed(this: &Ref<Self>) -> ReactiveUserControl<FeatureViewModel> {
        ReactiveUserControl::<FeatureViewModel>::from_control(this.clone())
    }

    /// The view as `IViewFor<FeatureViewModel>`.
    pub fn view_for_of(this: &Ref<Self>) -> Rc<dyn IViewForOf<FeatureViewModel>> {
        Self::typed(this).into_view_for_of()
    }

    /// The view model of the view (`ViewModel`).
    pub fn view_model(this: &Ref<Self>) -> Option<Rc<FeatureViewModel>> {
        Self::typed(this).view_model()
    }
}
