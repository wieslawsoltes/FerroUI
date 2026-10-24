//! Port of `Views/Pages/InitializedTriggerView` of the reference sample: the class of `pages/InitializedTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct InitializedTriggerView {
    base: UserControl,
}

user_control_class!(InitializedTriggerView);
ferro_class_info!(InitializedTriggerView { new: InitializedTriggerView::new });
xaml_class!(InitializedTriggerView, "/pages/InitializedTriggerView.xaml");

impl InitializedTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `InitializedTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    InitializedTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[InitializedTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "InitializedTrigger",
    reference: "Views/Pages/InitializedTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&InitializedTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
