//! Port of `Views/Pages/LoadedTriggerView` of the reference sample: the class of `pages/LoadedTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct LoadedTriggerView {
    base: UserControl,
}

user_control_class!(LoadedTriggerView);
ferro_class_info!(LoadedTriggerView { new: LoadedTriggerView::new });
xaml_class!(LoadedTriggerView, "/pages/LoadedTriggerView.xaml");

impl LoadedTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `LoadedTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    LoadedTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[LoadedTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "LoadedTrigger",
    reference: "Views/Pages/LoadedTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&LoadedTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
