//! Port of `Views/Pages/DelayedLoadBehaviorView` of the reference sample: the class of `pages/DelayedLoadBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DelayedLoadBehaviorView {
    base: UserControl,
}

user_control_class!(DelayedLoadBehaviorView);
ferro_class_info!(DelayedLoadBehaviorView { new: DelayedLoadBehaviorView::new });
xaml_class!(DelayedLoadBehaviorView, "/pages/DelayedLoadBehaviorView.xaml");

impl DelayedLoadBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `DelayedLoadBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DelayedLoadBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DelayedLoadBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "DelayedLoadBehavior",
    reference: "Views/Pages/DelayedLoadBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&DelayedLoadBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
