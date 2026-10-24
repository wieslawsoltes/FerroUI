//! Port of `Views/Pages/AddItemToItemsControlActionView` of the reference sample: the class of `pages/AddItemToItemsControlActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AddItemToItemsControlActionView {
    base: UserControl,
}

user_control_class!(AddItemToItemsControlActionView);
ferro_class_info!(AddItemToItemsControlActionView { new: AddItemToItemsControlActionView::new });
xaml_class!(AddItemToItemsControlActionView, "/pages/AddItemToItemsControlActionView.xaml");

impl AddItemToItemsControlActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `AddItemToItemsControlActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AddItemToItemsControlActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AddItemToItemsControlActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "AddItemToItemsControlAction",
    reference: "Views/Pages/AddItemToItemsControlActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&AddItemToItemsControlActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
