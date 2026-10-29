//! Port of `Views/Pages/ExecuteCommandBehaviorsView` of the reference sample: the class of `pages/ExecuteCommandBehaviorsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::execute_command_behaviors_view_model::ExecuteCommandBehaviorsViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ExecuteCommandBehaviorsView {
    base: UserControl,
}

user_control_class!(ExecuteCommandBehaviorsView);
ferro_class_info!(ExecuteCommandBehaviorsView {
    new: ExecuteCommandBehaviorsView::new
});
xaml_class!(
    ExecuteCommandBehaviorsView,
    "/pages/ExecuteCommandBehaviorsView.xaml"
);

impl ExecuteCommandBehaviorsView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ExecuteCommandBehaviorsView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(ExecuteCommandBehaviorsViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ExecuteCommandBehaviorsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ExecuteCommandBehaviorsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ExecuteCommand Behaviors",
    reference: "Views/Pages/ExecuteCommandBehaviorsView",
    create: Some(create),
    types: TYPES,
    classes: &[&ExecuteCommandBehaviorsView::XAML_CLASS],
    markup_types: &[<ExecuteCommandBehaviorsViewModel as MarkupTyped>::MARKUP],
    register_value_types,
};

/// What the untyped value conversions must know about the view model of the page.
fn register_value_types() {
    ValueTypes::register_reference::<ExecuteCommandBehaviorsViewModel>();
}
