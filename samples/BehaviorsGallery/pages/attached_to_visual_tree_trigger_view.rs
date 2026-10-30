//! Port of `Views/Pages/AttachedToVisualTreeTriggerView` of the reference sample: the class of `pages/AttachedToVisualTreeTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AttachedToVisualTreeTriggerView {
    base: UserControl,
}

user_control_class!(AttachedToVisualTreeTriggerView);
ferro_class_info!(AttachedToVisualTreeTriggerView { new: AttachedToVisualTreeTriggerView::new });
xaml_class!(AttachedToVisualTreeTriggerView, "/pages/AttachedToVisualTreeTriggerView.xaml");

impl AttachedToVisualTreeTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `AttachedToVisualTreeTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AttachedToVisualTreeTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AttachedToVisualTreeTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "AttachedToVisualTreeTrigger",
    reference: "Views/Pages/AttachedToVisualTreeTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&AttachedToVisualTreeTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
