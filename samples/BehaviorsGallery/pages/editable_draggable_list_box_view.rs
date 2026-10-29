//! Port of `Views/Pages/EditableDraggableListBoxView` of the reference sample: the class of `pages/EditableDraggableListBoxView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::view_models::DragAndDropSampleViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct EditableDraggableListBoxView {
    base: UserControl,
}

user_control_class!(EditableDraggableListBoxView);
ferro_class_info!(EditableDraggableListBoxView { new: EditableDraggableListBoxView::new });
xaml_class!(EditableDraggableListBoxView, "/pages/EditableDraggableListBoxView.xaml");

impl EditableDraggableListBoxView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `EditableDraggableListBoxView()`: the page has its own drag and drop view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(DragAndDropSampleViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    EditableDraggableListBoxView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[EditableDraggableListBoxView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "EditableDraggableListBox",
    reference: "Views/Pages/EditableDraggableListBoxView",
    create: Some(create),
    types: TYPES,
    classes: &[&EditableDraggableListBoxView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
