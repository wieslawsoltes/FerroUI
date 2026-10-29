//! Port of `Views/Pages/ButtonExecuteCommandOnKeyDownBehaviorView` of the reference sample: the class of `pages/ButtonExecuteCommandOnKeyDownBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::view_models::execute_command_behaviors_view_model::ExecuteCommandBehaviorsViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ButtonExecuteCommandOnKeyDownBehaviorView {
    base: UserControl,
}

user_control_class!(ButtonExecuteCommandOnKeyDownBehaviorView);
ferro_class_info!(ButtonExecuteCommandOnKeyDownBehaviorView {
    new: ButtonExecuteCommandOnKeyDownBehaviorView::new
});
xaml_class!(
    ButtonExecuteCommandOnKeyDownBehaviorView,
    "/pages/ButtonExecuteCommandOnKeyDownBehaviorView.xaml"
);

impl ButtonExecuteCommandOnKeyDownBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ButtonExecuteCommandOnKeyDownBehaviorView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(ExecuteCommandBehaviorsViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ButtonExecuteCommandOnKeyDownBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ButtonExecuteCommandOnKeyDownBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ButtonExecuteCommandOnKeyDownBehavior",
    reference: "Views/Pages/ButtonExecuteCommandOnKeyDownBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ButtonExecuteCommandOnKeyDownBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
