//! Port of `Views/Pages/InvokeCommandActionView` of the reference sample: the class of `pages/InvokeCommandActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct InvokeCommandActionView {
    base: UserControl,
}

user_control_class!(InvokeCommandActionView);
ferro_class_info!(InvokeCommandActionView {
    new: InvokeCommandActionView::new
});
xaml_class!(
    InvokeCommandActionView,
    "/pages/InvokeCommandActionView.xaml"
);

impl InvokeCommandActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `InvokeCommandActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    InvokeCommandActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[InvokeCommandActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "InvokeCommandAction",
    reference: "Views/Pages/InvokeCommandActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&InvokeCommandActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
