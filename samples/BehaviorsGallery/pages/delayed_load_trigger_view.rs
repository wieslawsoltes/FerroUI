//! Port of `Views/Pages/DelayedLoadTriggerView` of the reference sample: the class of `pages/DelayedLoadTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DelayedLoadTriggerView {
    base: UserControl,
}

user_control_class!(DelayedLoadTriggerView);
ferro_class_info!(DelayedLoadTriggerView { new: DelayedLoadTriggerView::new });
xaml_class!(DelayedLoadTriggerView, "/pages/DelayedLoadTriggerView.xaml");

impl DelayedLoadTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `DelayedLoadTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DelayedLoadTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DelayedLoadTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "DelayedLoadTrigger",
    reference: "Views/Pages/DelayedLoadTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&DelayedLoadTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
