//! Port of `Views/Pages/AnimationCompletedTriggerView` of the reference sample: the class of `pages/AnimationCompletedTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AnimationCompletedTriggerView {
    base: UserControl,
}

user_control_class!(AnimationCompletedTriggerView);
ferro_class_info!(AnimationCompletedTriggerView {
    new: AnimationCompletedTriggerView::new
});
xaml_class!(
    AnimationCompletedTriggerView,
    "/pages/AnimationCompletedTriggerView.xaml"
);

impl AnimationCompletedTriggerView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `AnimationCompletedTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AnimationCompletedTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AnimationCompletedTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Animation Completed Trigger",
    reference: "Views/Pages/AnimationCompletedTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&AnimationCompletedTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
