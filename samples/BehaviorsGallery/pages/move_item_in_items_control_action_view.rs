//! Port of `Views/Pages/MoveItemInItemsControlActionView` of the reference sample: the class of `pages/MoveItemInItemsControlActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct MoveItemInItemsControlActionView {
    base: UserControl,
}

user_control_class!(MoveItemInItemsControlActionView);
ferro_class_info!(MoveItemInItemsControlActionView { new: MoveItemInItemsControlActionView::new });
xaml_class!(MoveItemInItemsControlActionView, "/pages/MoveItemInItemsControlActionView.xaml");

impl MoveItemInItemsControlActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `MoveItemInItemsControlActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    MoveItemInItemsControlActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[MoveItemInItemsControlActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "MoveItemInItemsControlAction",
    reference: "Views/Pages/MoveItemInItemsControlActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&MoveItemInItemsControlActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
