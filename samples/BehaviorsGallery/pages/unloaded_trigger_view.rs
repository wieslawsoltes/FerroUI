//! Port of `Views/Pages/UnloadedTriggerView` of the reference sample: the class of `pages/UnloadedTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct UnloadedTriggerView {
    base: UserControl,
}

user_control_class!(UnloadedTriggerView);
ferro_class_info!(UnloadedTriggerView { new: UnloadedTriggerView::new });
xaml_class!(UnloadedTriggerView, "/pages/UnloadedTriggerView.xaml");

impl UnloadedTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `UnloadedTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    UnloadedTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[UnloadedTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "UnloadedTrigger",
    reference: "Views/Pages/UnloadedTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&UnloadedTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
