//! Port of `Views/Pages/GetClipboardDataActionView` of the reference sample: the class of `pages/GetClipboardDataActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct GetClipboardDataActionView {
    base: UserControl,
}

user_control_class!(GetClipboardDataActionView);
ferro_class_info!(GetClipboardDataActionView {
    new: GetClipboardDataActionView::new
});
xaml_class!(
    GetClipboardDataActionView,
    "/pages/GetClipboardDataActionView.xaml"
);

impl GetClipboardDataActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `GetClipboardDataActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    GetClipboardDataActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[GetClipboardDataActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "GetClipboardDataAction",
    reference: "Views/Pages/GetClipboardDataActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&GetClipboardDataActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
