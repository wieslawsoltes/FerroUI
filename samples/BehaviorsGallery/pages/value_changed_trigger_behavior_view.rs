//! Port of `Views/Pages/ValueChangedTriggerBehaviorView` of the reference sample: the class of `pages/ValueChangedTriggerBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ValueChangedTriggerBehaviorView {
    base: UserControl,
}

user_control_class!(ValueChangedTriggerBehaviorView);
ferro_class_info!(ValueChangedTriggerBehaviorView {
    new: ValueChangedTriggerBehaviorView::new
});
xaml_class!(
    ValueChangedTriggerBehaviorView,
    "/pages/ValueChangedTriggerBehaviorView.xaml"
);

impl ValueChangedTriggerBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ValueChangedTriggerBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ValueChangedTriggerBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ValueChangedTriggerBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ValueChangedTriggerBehavior",
    reference: "Views/Pages/ValueChangedTriggerBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ValueChangedTriggerBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
