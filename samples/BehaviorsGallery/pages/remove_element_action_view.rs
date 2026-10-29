//! Port of `Views/Pages/RemoveElementActionView` of the reference sample: the class of `pages/RemoveElementActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct RemoveElementActionView {
    base: UserControl,
}

user_control_class!(RemoveElementActionView);
ferro_class_info!(RemoveElementActionView { new: RemoveElementActionView::new });
xaml_class!(RemoveElementActionView, "/pages/RemoveElementActionView.xaml");

impl RemoveElementActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `RemoveElementActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    RemoveElementActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[RemoveElementActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "RemoveElementAction",
    reference: "Views/Pages/RemoveElementActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&RemoveElementActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
