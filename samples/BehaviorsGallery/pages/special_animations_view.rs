//! Port of `Views/Pages/SpecialAnimationsView` of the reference sample: the class of `pages/SpecialAnimationsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct SpecialAnimationsView {
    base: UserControl,
}

user_control_class!(SpecialAnimationsView);
ferro_class_info!(SpecialAnimationsView {
    new: SpecialAnimationsView::new
});
xaml_class!(SpecialAnimationsView, "/pages/SpecialAnimationsView.xaml");

impl SpecialAnimationsView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `SpecialAnimationsView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    SpecialAnimationsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[SpecialAnimationsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Special Animations",
    reference: "Views/Pages/SpecialAnimationsView",
    create: Some(create),
    types: TYPES,
    classes: &[&SpecialAnimationsView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
