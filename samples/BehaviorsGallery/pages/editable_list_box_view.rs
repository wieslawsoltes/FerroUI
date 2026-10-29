//! Port of `Views/Pages/EditableListBoxView` of the reference sample: the class of `pages/EditableListBoxView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct EditableListBoxView {
    base: UserControl,
}

user_control_class!(EditableListBoxView);
ferro_class_info!(EditableListBoxView { new: EditableListBoxView::new });
xaml_class!(EditableListBoxView, "/pages/EditableListBoxView.xaml");

impl EditableListBoxView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `EditableListBoxView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    EditableListBoxView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[EditableListBoxView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "EditableListBox",
    reference: "Views/Pages/EditableListBoxView",
    create: Some(create),
    types: TYPES,
    classes: &[&EditableListBoxView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
