//! Port of `Views/Pages/ContextDialogView` of the reference sample: the class of `pages/ContextDialogView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ContextDialogView {
    base: UserControl,
}

user_control_class!(ContextDialogView);
ferro_class_info!(ContextDialogView {
    new: ContextDialogView::new
});
xaml_class!(ContextDialogView, "/pages/ContextDialogView.xaml");

impl ContextDialogView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ContextDialogView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ContextDialogView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ContextDialogView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ContextDialog",
    reference: "Views/Pages/ContextDialogView",
    create: Some(create),
    types: TYPES,
    classes: &[&ContextDialogView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
