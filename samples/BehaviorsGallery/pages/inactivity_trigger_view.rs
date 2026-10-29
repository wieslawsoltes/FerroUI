//! Port of `Views/Pages/InactivityTriggerView` of the reference sample: the class of `pages/InactivityTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct InactivityTriggerView {
    base: UserControl,
}

user_control_class!(InactivityTriggerView);
ferro_class_info!(InactivityTriggerView { new: InactivityTriggerView::new });
xaml_class!(InactivityTriggerView, "/pages/InactivityTriggerView.xaml");

impl InactivityTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `InactivityTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    InactivityTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[InactivityTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "InactivityTrigger",
    reference: "Views/Pages/InactivityTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&InactivityTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
