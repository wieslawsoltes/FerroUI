//! Port of `Views/Pages/FocusNextElementActionView` of the reference sample: the class of `pages/FocusNextElementActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FocusNextElementActionView {
    base: UserControl,
}

user_control_class!(FocusNextElementActionView);
ferro_class_info!(FocusNextElementActionView { new: FocusNextElementActionView::new });
xaml_class!(FocusNextElementActionView, "/pages/FocusNextElementActionView.xaml");

impl FocusNextElementActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `FocusNextElementActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FocusNextElementActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FocusNextElementActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "FocusNextElementAction",
    reference: "Views/Pages/FocusNextElementActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&FocusNextElementActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
