//! Port of `Views/Pages/ScrollToOffsetActionView` of the reference sample: the class of `pages/ScrollToOffsetActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ScrollToOffsetActionView {
    base: UserControl,
}

user_control_class!(ScrollToOffsetActionView);
ferro_class_info!(ScrollToOffsetActionView { new: ScrollToOffsetActionView::new });
xaml_class!(ScrollToOffsetActionView, "/pages/ScrollToOffsetActionView.xaml");

impl ScrollToOffsetActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ScrollToOffsetActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ScrollToOffsetActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ScrollToOffsetActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ScrollToOffsetAction",
    reference: "Views/Pages/ScrollToOffsetActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&ScrollToOffsetActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
