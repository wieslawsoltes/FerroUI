//! Port of `Views/Pages/TaskCompletedTriggerView` of the reference sample: the class of `pages/TaskCompletedTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct TaskCompletedTriggerView {
    base: UserControl,
}

user_control_class!(TaskCompletedTriggerView);
ferro_class_info!(TaskCompletedTriggerView { new: TaskCompletedTriggerView::new });
xaml_class!(TaskCompletedTriggerView, "/pages/TaskCompletedTriggerView.xaml");

impl TaskCompletedTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `TaskCompletedTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    TaskCompletedTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[TaskCompletedTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "TaskCompletedTrigger",
    reference: "Views/Pages/TaskCompletedTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&TaskCompletedTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
