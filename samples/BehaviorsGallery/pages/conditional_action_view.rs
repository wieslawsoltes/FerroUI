//! Port of `Views/Pages/ConditionalActionView` of the reference sample: the class of `pages/ConditionalActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ConditionalActionView {
    base: UserControl,
}

user_control_class!(ConditionalActionView);
ferro_class_info!(ConditionalActionView { new: ConditionalActionView::new });
xaml_class!(ConditionalActionView, "/pages/ConditionalActionView.xaml");

impl ConditionalActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ConditionalActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ConditionalActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ConditionalActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ConditionalAction",
    reference: "Views/Pages/ConditionalActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&ConditionalActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
