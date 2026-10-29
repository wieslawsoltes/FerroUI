//! Port of `Views/Pages/EditableDoubleTappedListBoxView` of the reference sample: the class of `pages/EditableDoubleTappedListBoxView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct EditableDoubleTappedListBoxView {
    base: UserControl,
}

user_control_class!(EditableDoubleTappedListBoxView);
ferro_class_info!(EditableDoubleTappedListBoxView { new: EditableDoubleTappedListBoxView::new });
xaml_class!(EditableDoubleTappedListBoxView, "/pages/EditableDoubleTappedListBoxView.xaml");

impl EditableDoubleTappedListBoxView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `EditableDoubleTappedListBoxView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    EditableDoubleTappedListBoxView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[EditableDoubleTappedListBoxView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "EditableListBox DoubleTap",
    reference: "Views/Pages/EditableDoubleTappedListBoxView",
    create: Some(create),
    types: TYPES,
    classes: &[&EditableDoubleTappedListBoxView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
