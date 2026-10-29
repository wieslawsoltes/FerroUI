//! Port of `Views/Pages/ClickEventTriggerView` of the reference sample: the class of `pages/ClickEventTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ClickEventTriggerView {
    base: UserControl,
}

user_control_class!(ClickEventTriggerView);
ferro_class_info!(ClickEventTriggerView { new: ClickEventTriggerView::new });
xaml_class!(ClickEventTriggerView, "/pages/ClickEventTriggerView.xaml");

impl ClickEventTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ClickEventTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ClickEventTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ClickEventTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ClickEventTrigger",
    reference: "Views/Pages/ClickEventTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&ClickEventTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
