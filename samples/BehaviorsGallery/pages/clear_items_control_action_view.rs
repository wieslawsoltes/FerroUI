//! Port of `Views/Pages/ClearItemsControlActionView` of the reference sample: the class of `pages/ClearItemsControlActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ClearItemsControlActionView {
    base: UserControl,
}

user_control_class!(ClearItemsControlActionView);
ferro_class_info!(ClearItemsControlActionView { new: ClearItemsControlActionView::new });
xaml_class!(ClearItemsControlActionView, "/pages/ClearItemsControlActionView.xaml");

impl ClearItemsControlActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ClearItemsControlActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ClearItemsControlActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ClearItemsControlActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ClearItemsControlAction",
    reference: "Views/Pages/ClearItemsControlActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&ClearItemsControlActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
