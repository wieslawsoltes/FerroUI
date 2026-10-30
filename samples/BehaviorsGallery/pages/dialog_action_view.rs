//! Port of `Views/Pages/DialogActionView` of the reference sample: the class of `pages/DialogActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DialogActionView {
    base: UserControl,
}

user_control_class!(DialogActionView);
ferro_class_info!(DialogActionView {
    new: DialogActionView::new
});
xaml_class!(DialogActionView, "/pages/DialogActionView.xaml");

impl DialogActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `DialogActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DialogActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DialogActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Dialog Actions",
    reference: "Views/Pages/DialogActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&DialogActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
