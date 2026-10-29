//! Port of `Views/Pages/CustomBehaviorView` of the reference sample: the class of `pages/CustomBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct CustomBehaviorView {
    base: UserControl,
}

user_control_class!(CustomBehaviorView);
ferro_class_info!(CustomBehaviorView {
    new: CustomBehaviorView::new
});
xaml_class!(CustomBehaviorView, "/pages/CustomBehaviorView.xaml");

impl CustomBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `CustomBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    CustomBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[CustomBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Custom Behavior",
    reference: "Views/Pages/CustomBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&CustomBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
