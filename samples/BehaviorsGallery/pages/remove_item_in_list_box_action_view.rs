//! Port of `Views/Pages/RemoveItemInListBoxActionView` of the reference sample: the class of `pages/RemoveItemInListBoxActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct RemoveItemInListBoxActionView {
    base: UserControl,
}

user_control_class!(RemoveItemInListBoxActionView);
ferro_class_info!(RemoveItemInListBoxActionView { new: RemoveItemInListBoxActionView::new });
xaml_class!(RemoveItemInListBoxActionView, "/pages/RemoveItemInListBoxActionView.xaml");

impl RemoveItemInListBoxActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `RemoveItemInListBoxActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    RemoveItemInListBoxActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[RemoveItemInListBoxActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "RemoveItemInListBoxAction",
    reference: "Views/Pages/RemoveItemInListBoxActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&RemoveItemInListBoxActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
