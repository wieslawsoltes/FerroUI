//! Port of `Views/Pages/ContentControlFilesDropBehaviorView` of the reference sample: the class of `pages/ContentControlFilesDropBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ContentControlFilesDropBehaviorView {
    base: UserControl,
}

user_control_class!(ContentControlFilesDropBehaviorView);
ferro_class_info!(ContentControlFilesDropBehaviorView {
    new: ContentControlFilesDropBehaviorView::new
});
xaml_class!(
    ContentControlFilesDropBehaviorView,
    "/pages/ContentControlFilesDropBehaviorView.xaml"
);

impl ContentControlFilesDropBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ContentControlFilesDropBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ContentControlFilesDropBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ContentControlFilesDropBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ContentControlFilesDropBehavior",
    reference: "Views/Pages/ContentControlFilesDropBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ContentControlFilesDropBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
