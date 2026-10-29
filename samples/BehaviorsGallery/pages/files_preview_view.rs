//! Port of `Views/Pages/FilesPreviewView` of the reference sample: the class of `pages/FilesPreviewView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FilesPreviewView {
    base: UserControl,
}

user_control_class!(FilesPreviewView);
ferro_class_info!(FilesPreviewView {
    new: FilesPreviewView::new
});
xaml_class!(FilesPreviewView, "/pages/FilesPreviewView.xaml");

impl FilesPreviewView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `FilesPreviewView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FilesPreviewView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FilesPreviewView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Files Preview",
    reference: "Views/Pages/FilesPreviewView",
    create: Some(create),
    types: TYPES,
    classes: &[&FilesPreviewView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
