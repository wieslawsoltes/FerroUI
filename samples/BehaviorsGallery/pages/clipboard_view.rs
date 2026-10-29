//! Port of `Views/Pages/ClipboardView` of the reference sample: the class of `pages/ClipboardView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ClipboardView {
    base: UserControl,
}

user_control_class!(ClipboardView);
ferro_class_info!(ClipboardView {
    new: ClipboardView::new
});
xaml_class!(ClipboardView, "/pages/ClipboardView.xaml");

impl ClipboardView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ClipboardView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ClipboardView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ClipboardView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Clipboard",
    reference: "Views/Pages/ClipboardView",
    create: Some(create),
    types: TYPES,
    classes: &[&ClipboardView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
