//! Port of `Views/Pages/ButtonClickEventTriggerBehaviorView` of the reference sample: the class of `pages/ButtonClickEventTriggerBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ButtonClickEventTriggerBehaviorView {
    base: UserControl,
}

user_control_class!(ButtonClickEventTriggerBehaviorView);
ferro_class_info!(ButtonClickEventTriggerBehaviorView { new: ButtonClickEventTriggerBehaviorView::new });
xaml_class!(ButtonClickEventTriggerBehaviorView, "/pages/ButtonClickEventTriggerBehaviorView.xaml");

impl ButtonClickEventTriggerBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ButtonClickEventTriggerBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ButtonClickEventTriggerBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ButtonClickEventTriggerBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ButtonClickEventTriggerBehavior",
    reference: "Views/Pages/ButtonClickEventTriggerBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ButtonClickEventTriggerBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
