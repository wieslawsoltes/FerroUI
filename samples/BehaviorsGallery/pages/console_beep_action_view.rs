//! Port of `Views/Pages/ConsoleBeepActionView` of the reference sample: the class of `pages/ConsoleBeepActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ConsoleBeepActionView {
    base: UserControl,
}

user_control_class!(ConsoleBeepActionView);
ferro_class_info!(ConsoleBeepActionView { new: ConsoleBeepActionView::new });
xaml_class!(ConsoleBeepActionView, "/pages/ConsoleBeepActionView.xaml");

impl ConsoleBeepActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ConsoleBeepActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ConsoleBeepActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ConsoleBeepActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ConsoleBeepAction",
    reference: "Views/Pages/ConsoleBeepActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&ConsoleBeepActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
