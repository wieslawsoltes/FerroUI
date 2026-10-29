//! Port of `Views/Pages/FocusOnPointerPressedBehaviorView` of the reference sample: the class of `pages/FocusOnPointerPressedBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FocusOnPointerPressedBehaviorView {
    base: UserControl,
}

user_control_class!(FocusOnPointerPressedBehaviorView);
ferro_class_info!(FocusOnPointerPressedBehaviorView { new: FocusOnPointerPressedBehaviorView::new });
xaml_class!(FocusOnPointerPressedBehaviorView, "/pages/FocusOnPointerPressedBehaviorView.xaml");

impl FocusOnPointerPressedBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `FocusOnPointerPressedBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FocusOnPointerPressedBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FocusOnPointerPressedBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "FocusOnPointerPressedBehavior",
    reference: "Views/Pages/FocusOnPointerPressedBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&FocusOnPointerPressedBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
