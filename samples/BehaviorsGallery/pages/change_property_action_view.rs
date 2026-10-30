//! Port of `Views/Pages/ChangePropertyActionView` of the reference sample: the class of `pages/ChangePropertyActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ChangePropertyActionView {
    base: UserControl,
}

user_control_class!(ChangePropertyActionView);
ferro_class_info!(ChangePropertyActionView {
    new: ChangePropertyActionView::new
});
xaml_class!(
    ChangePropertyActionView,
    "/pages/ChangePropertyActionView.xaml"
);

impl ChangePropertyActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ChangePropertyActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ChangePropertyActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ChangePropertyActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ChangePropertyAction",
    reference: "Views/Pages/ChangePropertyActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&ChangePropertyActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
