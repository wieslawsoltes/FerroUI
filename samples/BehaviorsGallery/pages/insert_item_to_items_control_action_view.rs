//! Port of `Views/Pages/InsertItemToItemsControlActionView` of the reference sample: the class of `pages/InsertItemToItemsControlActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct InsertItemToItemsControlActionView {
    base: UserControl,
}

user_control_class!(InsertItemToItemsControlActionView);
ferro_class_info!(InsertItemToItemsControlActionView { new: InsertItemToItemsControlActionView::new });
xaml_class!(InsertItemToItemsControlActionView, "/pages/InsertItemToItemsControlActionView.xaml");

impl InsertItemToItemsControlActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `InsertItemToItemsControlActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    InsertItemToItemsControlActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[InsertItemToItemsControlActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "InsertItemToItemsControlAction",
    reference: "Views/Pages/InsertItemToItemsControlActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&InsertItemToItemsControlActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
