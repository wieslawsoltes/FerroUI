//! Port of `Views/Pages/DisposingTriggerView` of the reference sample: the class of `pages/DisposingTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::triggers::pointer_pressed_disposing_trigger::PointerPressedDisposingTrigger;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DisposingTriggerView {
    base: UserControl,
}

user_control_class!(DisposingTriggerView);
ferro_class_info!(DisposingTriggerView { new: DisposingTriggerView::new });
xaml_class!(DisposingTriggerView, "/pages/DisposingTriggerView.xaml");

impl DisposingTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `DisposingTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DisposingTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`): the page and the trigger of the sample it uses.
pub(crate) const TYPES: &[&TypeInfo] = &[DisposingTriggerView::TYPE, PointerPressedDisposingTrigger::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "DisposingTrigger",
    reference: "Views/Pages/DisposingTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&DisposingTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
