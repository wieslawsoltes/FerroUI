//! Port of `Views/Pages/DraggableView` of the reference sample: the class of `pages/DraggableView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::draggable_view_model::DraggableViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DraggableView {
    base: UserControl,
}

user_control_class!(DraggableView);
ferro_class_info!(DraggableView {
    new: DraggableView::new
});
xaml_class!(DraggableView, "/pages/DraggableView.xaml");

impl DraggableView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `DraggableView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(DraggableViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DraggableView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DraggableView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Draggable",
    reference: "Views/Pages/DraggableView",
    create: Some(create),
    types: TYPES,
    classes: &[&DraggableView::XAML_CLASS],
    markup_types: &[<DraggableViewModel as MarkupTyped>::MARKUP],
    register_value_types,
};

/// What the untyped value conversions must know about the view model of the page.
fn register_value_types() {
    ValueTypes::register_reference::<DraggableViewModel>();
}
