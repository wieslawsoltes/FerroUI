//! Port of `Views/Pages/BindingTriggerBehaviorView` of the reference sample: the class of `pages/BindingTriggerBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct BindingTriggerBehaviorView {
    base: UserControl,
}

user_control_class!(BindingTriggerBehaviorView);
ferro_class_info!(BindingTriggerBehaviorView { new: BindingTriggerBehaviorView::new });
xaml_class!(BindingTriggerBehaviorView, "/pages/BindingTriggerBehaviorView.xaml");

impl BindingTriggerBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `BindingTriggerBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    BindingTriggerBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[BindingTriggerBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "BindingTriggerBehavior",
    reference: "Views/Pages/BindingTriggerBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&BindingTriggerBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
