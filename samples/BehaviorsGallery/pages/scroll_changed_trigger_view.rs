//! Port of `Views/Pages/ScrollChangedTriggerView` of the reference sample: the class of `pages/ScrollChangedTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ScrollChangedTriggerView {
    base: UserControl,
}

user_control_class!(ScrollChangedTriggerView);
ferro_class_info!(ScrollChangedTriggerView { new: ScrollChangedTriggerView::new });
xaml_class!(ScrollChangedTriggerView, "/pages/ScrollChangedTriggerView.xaml");

impl ScrollChangedTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ScrollChangedTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ScrollChangedTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ScrollChangedTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ScrollChangedTrigger",
    reference: "Views/Pages/ScrollChangedTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&ScrollChangedTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
