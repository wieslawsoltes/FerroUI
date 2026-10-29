//! Port of `Views/Pages/UploadFileView` of the reference sample: the class of `pages/UploadFileView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct UploadFileView {
    base: UserControl,
}

user_control_class!(UploadFileView);
ferro_class_info!(UploadFileView {
    new: UploadFileView::new
});
xaml_class!(UploadFileView, "/pages/UploadFileView.xaml");

impl UploadFileView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `UploadFileView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    UploadFileView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[UploadFileView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Upload File",
    reference: "Views/Pages/UploadFileView",
    create: Some(create),
    types: TYPES,
    classes: &[&UploadFileView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
