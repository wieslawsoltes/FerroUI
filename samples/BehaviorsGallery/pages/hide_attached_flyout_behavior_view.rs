//! Port of `Views/Pages/HideAttachedFlyoutBehaviorView` of the reference sample: the class of `pages/HideAttachedFlyoutBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct HideAttachedFlyoutBehaviorView {
    base: UserControl,
}

user_control_class!(HideAttachedFlyoutBehaviorView);
ferro_class_info!(HideAttachedFlyoutBehaviorView { new: HideAttachedFlyoutBehaviorView::new });
xaml_class!(HideAttachedFlyoutBehaviorView, "/pages/HideAttachedFlyoutBehaviorView.xaml");

impl HideAttachedFlyoutBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `HideAttachedFlyoutBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    HideAttachedFlyoutBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[HideAttachedFlyoutBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "HideAttachedFlyoutBehavior",
    reference: "Views/Pages/HideAttachedFlyoutBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&HideAttachedFlyoutBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
