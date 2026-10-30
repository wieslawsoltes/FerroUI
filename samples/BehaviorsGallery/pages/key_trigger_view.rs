//! Port of `Views/Pages/KeyTriggerView` of the reference sample: the class of `pages/KeyTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct KeyTriggerView {
    base: UserControl,
}

user_control_class!(KeyTriggerView);
ferro_class_info!(KeyTriggerView { new: KeyTriggerView::new });
xaml_class!(KeyTriggerView, "/pages/KeyTriggerView.xaml");

impl KeyTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `KeyTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    KeyTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[KeyTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "KeyTrigger",
    reference: "Views/Pages/KeyTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&KeyTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
