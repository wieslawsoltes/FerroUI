//! Port of `Views/Pages/FileSystemView` of the reference sample: the class of `pages/FileSystemView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::file_system_view_model::FileSystemViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FileSystemView {
    base: UserControl,
}

user_control_class!(FileSystemView);
ferro_class_info!(FileSystemView { new: FileSystemView::new });
xaml_class!(FileSystemView, "/pages/FileSystemView.xaml");

impl FileSystemView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `FileSystemView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FileSystemView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FileSystemView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "FileSystem",
    reference: "Views/Pages/FileSystemView",
    create: Some(create),
    types: TYPES,
    classes: &[&FileSystemView::XAML_CLASS],
    markup_types: &[<FileSystemViewModel as MarkupTyped>::MARKUP],
    register_value_types,
};

/// What the untyped value conversions must know about the view model of the page (the document
/// creates it as its data context).
fn register_value_types() {
    ValueTypes::register_reference::<FileSystemViewModel>();
}
