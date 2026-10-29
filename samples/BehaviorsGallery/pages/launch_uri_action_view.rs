//! Port of `Views/Pages/LaunchUriActionView` of the reference sample: the class of `pages/LaunchUriActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct LaunchUriActionView {
    base: UserControl,
}

user_control_class!(LaunchUriActionView);
ferro_class_info!(LaunchUriActionView { new: LaunchUriActionView::new });
xaml_class!(LaunchUriActionView, "/pages/LaunchUriActionView.xaml");

impl LaunchUriActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `LaunchUriActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    LaunchUriActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[LaunchUriActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "LaunchUriAction",
    reference: "Views/Pages/LaunchUriActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&LaunchUriActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
