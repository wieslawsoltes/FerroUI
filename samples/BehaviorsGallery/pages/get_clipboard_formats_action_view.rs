//! Port of `Views/Pages/GetClipboardFormatsActionView` of the reference sample: the class of `pages/GetClipboardFormatsActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct GetClipboardFormatsActionView {
    base: UserControl,
}

user_control_class!(GetClipboardFormatsActionView);
ferro_class_info!(GetClipboardFormatsActionView {
    new: GetClipboardFormatsActionView::new
});
xaml_class!(
    GetClipboardFormatsActionView,
    "/pages/GetClipboardFormatsActionView.xaml"
);

impl GetClipboardFormatsActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `GetClipboardFormatsActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    GetClipboardFormatsActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[GetClipboardFormatsActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "GetClipboardFormatsAction",
    reference: "Views/Pages/GetClipboardFormatsActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&GetClipboardFormatsActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
