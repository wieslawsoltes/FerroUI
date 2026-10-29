//! Port of `Views/Pages/DragAndDropView` of the reference sample: the class of `pages/DragAndDropView.xaml`.

use crate::behaviors::managed_items_list_box_drop_handler::ManagedItemsListBoxDropHandler;
use crate::behaviors::nodes_list_box_drop_handler::NodesListBoxDropHandler;
use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::DragAndDropSampleViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};
use ferroui_xaml_interactions_draganddrop::IDropHandler;
use std::rc::Rc;

#[repr(C)]
pub struct DragAndDropView {
    base: UserControl,
}

user_control_class!(DragAndDropView);
ferro_class_info!(DragAndDropView {
    new: DragAndDropView::new
});
xaml_class!(DragAndDropView, "/pages/DragAndDropView.xaml");

impl DragAndDropView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `DragAndDropView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(DragAndDropSampleViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DragAndDropView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DragAndDropView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Drag and Drop",
    reference: "Views/Pages/DragAndDropView",
    create: Some(create),
    types: TYPES,
    classes: &[&DragAndDropView::XAML_CLASS],
    markup_types: &[
        <ManagedItemsListBoxDropHandler as MarkupTyped>::MARKUP,
        <NodesListBoxDropHandler as MarkupTyped>::MARKUP,
    ],
    register_value_types,
};

/// What the untyped value conversions must know about the drop handlers of the page: the handlers
/// as drop handlers.
fn register_value_types() {
    ValueTypes::register_reference::<ManagedItemsListBoxDropHandler>();
    ValueTypes::register_cast::<Rc<ManagedItemsListBoxDropHandler>, Rc<dyn IDropHandler>>(
        |handler| handler.clone(),
    );
    ValueTypes::register_reference::<NodesListBoxDropHandler>();
    ValueTypes::register_cast::<Rc<NodesListBoxDropHandler>, Rc<dyn IDropHandler>>(|handler| {
        handler.clone()
    });
}
