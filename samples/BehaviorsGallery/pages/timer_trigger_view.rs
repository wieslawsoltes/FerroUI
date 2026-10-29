//! Port of `Views/Pages/TimerTriggerView` of the reference sample: the class of `pages/TimerTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct TimerTriggerView {
    base: UserControl,
}

user_control_class!(TimerTriggerView);
ferro_class_info!(TimerTriggerView {
    new: TimerTriggerView::new
});
xaml_class!(TimerTriggerView, "/pages/TimerTriggerView.xaml");

impl TimerTriggerView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `TimerTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    TimerTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[TimerTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "TimerTrigger",
    reference: "Views/Pages/TimerTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&TimerTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
