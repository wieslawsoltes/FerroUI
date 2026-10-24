//! Port of `Views/ReactiveUI/InteractionTriggerBehaviorView` of the reference sample: the class of `pages/InteractionTriggerBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::reactive_ui::interaction_trigger_behavior_view_model::InteractionTriggerBehaviorViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct InteractionTriggerBehaviorView {
    base: UserControl,
}

user_control_class!(InteractionTriggerBehaviorView);
ferro_class_info!(InteractionTriggerBehaviorView {
    new: InteractionTriggerBehaviorView::new
});
xaml_class!(
    InteractionTriggerBehaviorView,
    "/pages/InteractionTriggerBehaviorView.xaml"
);

impl InteractionTriggerBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `InteractionTriggerBehaviorView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(
            InteractionTriggerBehaviorViewModel::new() as BoxedValue
        ));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    InteractionTriggerBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[InteractionTriggerBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "InteractionTriggerBehavior",
    reference: "Views/ReactiveUI/InteractionTriggerBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&InteractionTriggerBehaviorView::XAML_CLASS],
    markup_types: &[<InteractionTriggerBehaviorViewModel as MarkupTyped>::MARKUP],
    register_value_types,
};

/// What the untyped value conversions must know about the view model of the page.
fn register_value_types() {
    ValueTypes::register_reference::<InteractionTriggerBehaviorViewModel>();
}
