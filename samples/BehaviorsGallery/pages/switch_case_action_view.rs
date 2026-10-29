//! Port of `Views/Pages/SwitchCaseActionView` of the reference sample: the class of `pages/SwitchCaseActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct SwitchCaseActionView {
    base: UserControl,
}

user_control_class!(SwitchCaseActionView);
ferro_class_info!(SwitchCaseActionView { new: SwitchCaseActionView::new });
xaml_class!(SwitchCaseActionView, "/pages/SwitchCaseActionView.xaml");

impl SwitchCaseActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `SwitchCaseActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    SwitchCaseActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[SwitchCaseActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "SwitchCaseAction",
    reference: "Views/Pages/SwitchCaseActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&SwitchCaseActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
