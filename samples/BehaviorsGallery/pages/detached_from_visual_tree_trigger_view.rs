//! Port of `Views/Pages/DetachedFromVisualTreeTriggerView` of the reference sample: the class of `pages/DetachedFromVisualTreeTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DetachedFromVisualTreeTriggerView {
    base: UserControl,
}

user_control_class!(DetachedFromVisualTreeTriggerView);
ferro_class_info!(DetachedFromVisualTreeTriggerView { new: DetachedFromVisualTreeTriggerView::new });
xaml_class!(DetachedFromVisualTreeTriggerView, "/pages/DetachedFromVisualTreeTriggerView.xaml");

impl DetachedFromVisualTreeTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `DetachedFromVisualTreeTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DetachedFromVisualTreeTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DetachedFromVisualTreeTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "DetachedFromVisualTreeTrigger",
    reference: "Views/Pages/DetachedFromVisualTreeTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&DetachedFromVisualTreeTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
