//! Port of `Views/Pages/FluidMoveBehaviorView` of the reference sample: the class of `pages/FluidMoveBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::fluid_move_behavior_view_model::FluidMoveBehaviorViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FluidMoveBehaviorView {
    base: UserControl,
}

user_control_class!(FluidMoveBehaviorView);
ferro_class_info!(FluidMoveBehaviorView {
    new: FluidMoveBehaviorView::new
});
xaml_class!(FluidMoveBehaviorView, "/pages/FluidMoveBehaviorView.xaml");

impl FluidMoveBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `FluidMoveBehaviorView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(FluidMoveBehaviorViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FluidMoveBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FluidMoveBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "FluidMoveBehavior",
    reference: "Views/Pages/FluidMoveBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&FluidMoveBehaviorView::XAML_CLASS],
    markup_types: &[<FluidMoveBehaviorViewModel as MarkupTyped>::MARKUP],
    register_value_types,
};

/// What the untyped value conversions must know about the view model of the page.
fn register_value_types() {
    ValueTypes::register_reference::<FluidMoveBehaviorViewModel>();
}
