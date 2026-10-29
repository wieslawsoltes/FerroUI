//! Port of `Views/Pages/AttachedToLogicalTreeTriggerView` of the reference sample: the class of `pages/AttachedToLogicalTreeTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AttachedToLogicalTreeTriggerView {
    base: UserControl,
}

user_control_class!(AttachedToLogicalTreeTriggerView);
ferro_class_info!(AttachedToLogicalTreeTriggerView { new: AttachedToLogicalTreeTriggerView::new });
xaml_class!(AttachedToLogicalTreeTriggerView, "/pages/AttachedToLogicalTreeTriggerView.xaml");

impl AttachedToLogicalTreeTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `AttachedToLogicalTreeTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AttachedToLogicalTreeTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AttachedToLogicalTreeTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "AttachedToLogicalTreeTrigger",
    reference: "Views/Pages/AttachedToLogicalTreeTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&AttachedToLogicalTreeTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
