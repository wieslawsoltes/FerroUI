//! Port of `Views/Pages/RunAnimationTriggerView` of the reference sample: the class of `pages/RunAnimationTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::view_models::custom_animator_view_model::CustomAnimatorViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct RunAnimationTriggerView {
    base: UserControl,
}

user_control_class!(RunAnimationTriggerView);
ferro_class_info!(RunAnimationTriggerView {
    new: RunAnimationTriggerView::new
});
xaml_class!(
    RunAnimationTriggerView,
    "/pages/RunAnimationTriggerView.xaml"
);

impl RunAnimationTriggerView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `RunAnimationTriggerView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(CustomAnimatorViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    RunAnimationTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[RunAnimationTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "RunAnimationTrigger",
    reference: "Views/Pages/RunAnimationTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&RunAnimationTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
