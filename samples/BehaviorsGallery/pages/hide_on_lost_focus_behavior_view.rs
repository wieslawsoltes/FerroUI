//! Port of `Views/Pages/HideOnLostFocusBehaviorView` of the reference sample: the class of `pages/HideOnLostFocusBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct HideOnLostFocusBehaviorView {
    base: UserControl,
}

user_control_class!(HideOnLostFocusBehaviorView);
ferro_class_info!(HideOnLostFocusBehaviorView { new: HideOnLostFocusBehaviorView::new });
xaml_class!(HideOnLostFocusBehaviorView, "/pages/HideOnLostFocusBehaviorView.xaml");

impl HideOnLostFocusBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `HideOnLostFocusBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    HideOnLostFocusBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[HideOnLostFocusBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "HideOnLostFocusBehavior",
    reference: "Views/Pages/HideOnLostFocusBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&HideOnLostFocusBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
