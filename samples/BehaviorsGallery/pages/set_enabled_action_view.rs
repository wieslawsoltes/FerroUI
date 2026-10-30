//! Port of `Views/Pages/SetEnabledActionView` of the reference sample: the class of `pages/SetEnabledActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct SetEnabledActionView {
    base: UserControl,
}

user_control_class!(SetEnabledActionView);
ferro_class_info!(SetEnabledActionView {
    new: SetEnabledActionView::new
});
xaml_class!(SetEnabledActionView, "/pages/SetEnabledActionView.xaml");

impl SetEnabledActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `SetEnabledActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    SetEnabledActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[SetEnabledActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "SetEnabledAction",
    reference: "Views/Pages/SetEnabledActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&SetEnabledActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
