//! Port of `Views/Pages/AddRemoveClassActionView` of the reference sample: the class of `pages/AddRemoveClassActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AddRemoveClassActionView {
    base: UserControl,
}

user_control_class!(AddRemoveClassActionView);
ferro_class_info!(AddRemoveClassActionView {
    new: AddRemoveClassActionView::new
});
xaml_class!(
    AddRemoveClassActionView,
    "/pages/AddRemoveClassActionView.xaml"
);

impl AddRemoveClassActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `AddRemoveClassActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AddRemoveClassActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AddRemoveClassActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Add/RemoveClassAction",
    reference: "Views/Pages/AddRemoveClassActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&AddRemoveClassActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
