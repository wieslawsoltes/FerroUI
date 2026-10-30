//! Port of `Views/Pages/EditableDoubleTappedTreeViewView` of the reference sample: the class of `pages/EditableDoubleTappedTreeViewView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct EditableDoubleTappedTreeViewView {
    base: UserControl,
}

user_control_class!(EditableDoubleTappedTreeViewView);
ferro_class_info!(EditableDoubleTappedTreeViewView { new: EditableDoubleTappedTreeViewView::new });
xaml_class!(EditableDoubleTappedTreeViewView, "/pages/EditableDoubleTappedTreeViewView.xaml");

impl EditableDoubleTappedTreeViewView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `EditableDoubleTappedTreeViewView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    EditableDoubleTappedTreeViewView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[EditableDoubleTappedTreeViewView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "EditableTree DoubleTap",
    reference: "Views/Pages/EditableDoubleTappedTreeViewView",
    create: Some(create),
    types: TYPES,
    classes: &[&EditableDoubleTappedTreeViewView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
