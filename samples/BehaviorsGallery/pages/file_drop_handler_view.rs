//! Port of `Views/Pages/FileDropHandlerView` of the reference sample: the class of `pages/FileDropHandlerView.xaml`.

use crate::behaviors::files_drop_handler::FilesDropHandler;
use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};
use ferroui_xaml_interactions_draganddrop::IDropHandler;
use std::rc::Rc;

#[repr(C)]
pub struct FileDropHandlerView {
    base: UserControl,
}

user_control_class!(FileDropHandlerView);
ferro_class_info!(FileDropHandlerView {
    new: FileDropHandlerView::new
});
xaml_class!(FileDropHandlerView, "/pages/FileDropHandlerView.xaml");

impl FileDropHandlerView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `FileDropHandlerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FileDropHandlerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FileDropHandlerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "File Drop Handler",
    reference: "Views/Pages/FileDropHandlerView",
    create: Some(create),
    types: TYPES,
    classes: &[&FileDropHandlerView::XAML_CLASS],
    markup_types: &[<FilesDropHandler as MarkupTyped>::MARKUP],
    register_value_types,
};

/// What the untyped value conversions must know about the drop handler of the page: the handler
/// as a drop handler.
fn register_value_types() {
    ValueTypes::register_reference::<FilesDropHandler>();
    ValueTypes::register_cast::<Rc<FilesDropHandler>, Rc<dyn IDropHandler>>(|handler| {
        handler.clone()
    });
}
