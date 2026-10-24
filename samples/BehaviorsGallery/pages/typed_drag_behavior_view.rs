//! Port of `Views/Pages/TypedDragBehaviorView` of the reference sample: the class of `pages/TypedDragBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::view_models::DragAndDropSampleViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct TypedDragBehaviorView {
    base: UserControl,
}

user_control_class!(TypedDragBehaviorView);
ferro_class_info!(TypedDragBehaviorView {
    new: TypedDragBehaviorView::new
});
xaml_class!(TypedDragBehaviorView, "/pages/TypedDragBehaviorView.xaml");

impl TypedDragBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `TypedDragBehaviorView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(DragAndDropSampleViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    TypedDragBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[TypedDragBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "TypedDragBehavior",
    reference: "Views/Pages/TypedDragBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&TypedDragBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
