//! Port of `Views/Pages/RemoveItemInItemsControlActionView` of the reference sample: the class of `pages/RemoveItemInItemsControlActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct RemoveItemInItemsControlActionView {
    base: UserControl,
}

user_control_class!(RemoveItemInItemsControlActionView);
ferro_class_info!(RemoveItemInItemsControlActionView { new: RemoveItemInItemsControlActionView::new });
xaml_class!(RemoveItemInItemsControlActionView, "/pages/RemoveItemInItemsControlActionView.xaml");

impl RemoveItemInItemsControlActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `RemoveItemInItemsControlActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    RemoveItemInItemsControlActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[RemoveItemInItemsControlActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "RemoveItemInItemsControlAction",
    reference: "Views/Pages/RemoveItemInItemsControlActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&RemoveItemInItemsControlActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
