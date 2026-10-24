//! Port of `Views/ReactiveUI/ObservableTriggerBehaviorView` of the reference sample: the class of
//! `pages/ObservableTriggerBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ObservableTriggerBehaviorView {
    base: UserControl,
}

user_control_class!(ObservableTriggerBehaviorView);
ferro_class_info!(ObservableTriggerBehaviorView { new: ObservableTriggerBehaviorView::new });
xaml_class!(ObservableTriggerBehaviorView, "/pages/ObservableTriggerBehaviorView.xaml");

impl ObservableTriggerBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ObservableTriggerBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ObservableTriggerBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ObservableTriggerBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Observable Trigger",
    reference: "Views/ReactiveUI/ObservableTriggerBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ObservableTriggerBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
