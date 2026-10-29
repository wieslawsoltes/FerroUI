//! Port of `Views/Pages/FlyoutBindingView` of the reference sample: the class of `pages/FlyoutBindingView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FlyoutBindingView {
    base: UserControl,
}

user_control_class!(FlyoutBindingView);
ferro_class_info!(FlyoutBindingView {
    new: FlyoutBindingView::new
});
xaml_class!(FlyoutBindingView, "/pages/FlyoutBindingView.xaml");

impl FlyoutBindingView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `FlyoutBindingView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FlyoutBindingView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FlyoutBindingView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Flyout Binding",
    reference: "Views/Pages/FlyoutBindingView",
    create: Some(create),
    types: TYPES,
    classes: &[&FlyoutBindingView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
