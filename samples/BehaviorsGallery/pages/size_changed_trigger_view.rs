//! Port of `Views/Pages/SizeChangedTriggerView` of the reference sample: the class of `pages/SizeChangedTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct SizeChangedTriggerView {
    base: UserControl,
}

user_control_class!(SizeChangedTriggerView);
ferro_class_info!(SizeChangedTriggerView { new: SizeChangedTriggerView::new });
xaml_class!(SizeChangedTriggerView, "/pages/SizeChangedTriggerView.xaml");

impl SizeChangedTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `SizeChangedTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    SizeChangedTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[SizeChangedTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "SizeChangedTrigger",
    reference: "Views/Pages/SizeChangedTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&SizeChangedTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
