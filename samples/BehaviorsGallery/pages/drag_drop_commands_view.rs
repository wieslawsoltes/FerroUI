//! Port of `Views/Pages/DragDropCommandsView` of the reference sample: the class of `pages/DragDropCommandsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::drag_drop_commands_view_model::DragDropCommandsViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DragDropCommandsView {
    base: UserControl,
}

user_control_class!(DragDropCommandsView);
ferro_class_info!(DragDropCommandsView {
    new: DragDropCommandsView::new
});
xaml_class!(DragDropCommandsView, "/pages/DragDropCommandsView.xaml");

impl DragDropCommandsView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `DragDropCommandsView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(DragDropCommandsViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DragDropCommandsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DragDropCommandsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "DragDrop Commands",
    reference: "Views/Pages/DragDropCommandsView",
    create: Some(create),
    types: TYPES,
    classes: &[&DragDropCommandsView::XAML_CLASS],
    markup_types: &[<DragDropCommandsViewModel as MarkupTyped>::MARKUP],
    register_value_types,
};

/// What the untyped value conversions must know about the view model of the page.
fn register_value_types() {
    ValueTypes::register_reference::<DragDropCommandsViewModel>();
}
