//! Port of `Views/Pages/KeyGestureTriggerView` of the reference sample: the class of `pages/KeyGestureTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::key_gesture_trigger_view_model::KeyGestureTriggerViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct KeyGestureTriggerView {
    base: UserControl,
}

user_control_class!(KeyGestureTriggerView);
ferro_class_info!(KeyGestureTriggerView {
    new: KeyGestureTriggerView::new
});
xaml_class!(KeyGestureTriggerView, "/pages/KeyGestureTriggerView.xaml");

impl KeyGestureTriggerView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `KeyGestureTriggerView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(KeyGestureTriggerViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    KeyGestureTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[KeyGestureTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "KeyGestureTrigger",
    reference: "Views/Pages/KeyGestureTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&KeyGestureTriggerView::XAML_CLASS],
    markup_types: &[<KeyGestureTriggerViewModel as MarkupTyped>::MARKUP],
    register_value_types,
};

/// What the untyped value conversions must know about the view model of the page.
fn register_value_types() {
    ValueTypes::register_reference::<KeyGestureTriggerViewModel>();
}
