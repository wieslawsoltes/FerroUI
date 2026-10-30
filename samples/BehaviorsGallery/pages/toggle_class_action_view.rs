//! Port of `Views/Pages/ToggleClassActionView` of the reference sample: the class of `pages/ToggleClassActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ToggleClassActionView {
    base: UserControl,
}

user_control_class!(ToggleClassActionView);
ferro_class_info!(ToggleClassActionView { new: ToggleClassActionView::new });
xaml_class!(ToggleClassActionView, "/pages/ToggleClassActionView.xaml");

impl ToggleClassActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ToggleClassActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ToggleClassActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ToggleClassActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ToggleClassAction",
    reference: "Views/Pages/ToggleClassActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&ToggleClassActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
