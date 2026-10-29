//! Port of `Views/Pages/DetachedFromLogicalTreeTriggerView` of the reference sample: the class of `pages/DetachedFromLogicalTreeTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DetachedFromLogicalTreeTriggerView {
    base: UserControl,
}

user_control_class!(DetachedFromLogicalTreeTriggerView);
ferro_class_info!(DetachedFromLogicalTreeTriggerView { new: DetachedFromLogicalTreeTriggerView::new });
xaml_class!(DetachedFromLogicalTreeTriggerView, "/pages/DetachedFromLogicalTreeTriggerView.xaml");

impl DetachedFromLogicalTreeTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `DetachedFromLogicalTreeTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DetachedFromLogicalTreeTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DetachedFromLogicalTreeTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "DetachedFromLogicalTreeTrigger",
    reference: "Views/Pages/DetachedFromLogicalTreeTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&DetachedFromLogicalTreeTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
