//! Port of `Views/Pages/ThemeVariantTriggerView` of the reference sample: the class of `pages/ThemeVariantTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ThemeVariantTriggerView {
    base: UserControl,
}

user_control_class!(ThemeVariantTriggerView);
ferro_class_info!(ThemeVariantTriggerView { new: ThemeVariantTriggerView::new });
xaml_class!(ThemeVariantTriggerView, "/pages/ThemeVariantTriggerView.xaml");

impl ThemeVariantTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ThemeVariantTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ThemeVariantTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ThemeVariantTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ThemeVariantTrigger",
    reference: "Views/Pages/ThemeVariantTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&ThemeVariantTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
