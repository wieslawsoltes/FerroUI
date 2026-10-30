//! Port of `Views/Pages/HideOnKeyPressedBehaviorView` of the reference sample: the class of `pages/HideOnKeyPressedBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct HideOnKeyPressedBehaviorView {
    base: UserControl,
}

user_control_class!(HideOnKeyPressedBehaviorView);
ferro_class_info!(HideOnKeyPressedBehaviorView { new: HideOnKeyPressedBehaviorView::new });
xaml_class!(HideOnKeyPressedBehaviorView, "/pages/HideOnKeyPressedBehaviorView.xaml");

impl HideOnKeyPressedBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `HideOnKeyPressedBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    HideOnKeyPressedBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[HideOnKeyPressedBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "HideOnKeyPressedBehavior",
    reference: "Views/Pages/HideOnKeyPressedBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&HideOnKeyPressedBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
