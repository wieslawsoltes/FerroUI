//! Port of `Views/Pages/CustomActionView` of the reference sample: the class of `pages/CustomActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct CustomActionView {
    base: UserControl,
}

user_control_class!(CustomActionView);
ferro_class_info!(CustomActionView {
    new: CustomActionView::new
});
xaml_class!(CustomActionView, "/pages/CustomActionView.xaml");

impl CustomActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `CustomActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    CustomActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[CustomActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Custom Action",
    reference: "Views/Pages/CustomActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&CustomActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
