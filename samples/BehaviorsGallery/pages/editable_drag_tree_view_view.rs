//! Port of `Views/Pages/EditableDragTreeViewView` of the reference sample: the class of `pages/EditableDragTreeViewView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::view_models::DragAndDropSampleViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct EditableDragTreeViewView {
    base: UserControl,
}

user_control_class!(EditableDragTreeViewView);
ferro_class_info!(EditableDragTreeViewView { new: EditableDragTreeViewView::new });
xaml_class!(EditableDragTreeViewView, "/pages/EditableDragTreeViewView.xaml");

impl EditableDragTreeViewView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `EditableDragTreeViewView()`: the page has its own drag and drop view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(DragAndDropSampleViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    EditableDragTreeViewView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[EditableDragTreeViewView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Editable Drag TreeView",
    reference: "Views/Pages/EditableDragTreeViewView",
    create: Some(create),
    types: TYPES,
    classes: &[&EditableDragTreeViewView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
