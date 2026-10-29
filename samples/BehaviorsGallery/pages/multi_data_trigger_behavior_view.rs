//! Port of `Views/Pages/MultiDataTriggerBehaviorView` of the reference sample: the class of `pages/MultiDataTriggerBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct MultiDataTriggerBehaviorView {
    base: UserControl,
}

user_control_class!(MultiDataTriggerBehaviorView);
ferro_class_info!(MultiDataTriggerBehaviorView { new: MultiDataTriggerBehaviorView::new });
xaml_class!(MultiDataTriggerBehaviorView, "/pages/MultiDataTriggerBehaviorView.xaml");

impl MultiDataTriggerBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `MultiDataTriggerBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    MultiDataTriggerBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[MultiDataTriggerBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "MultiDataTriggerBehavior",
    reference: "Views/Pages/MultiDataTriggerBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&MultiDataTriggerBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
