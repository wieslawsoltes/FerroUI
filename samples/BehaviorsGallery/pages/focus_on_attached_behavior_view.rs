//! Port of `Views/Pages/FocusOnAttachedBehaviorView` of the reference sample: the class of `pages/FocusOnAttachedBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FocusOnAttachedBehaviorView {
    base: UserControl,
}

user_control_class!(FocusOnAttachedBehaviorView);
ferro_class_info!(FocusOnAttachedBehaviorView {
    new: FocusOnAttachedBehaviorView::new
});
xaml_class!(
    FocusOnAttachedBehaviorView,
    "/pages/FocusOnAttachedBehaviorView.xaml"
);

impl FocusOnAttachedBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `FocusOnAttachedBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FocusOnAttachedBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FocusOnAttachedBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "FocusOnAttachedBehavior",
    reference: "Views/Pages/FocusOnAttachedBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&FocusOnAttachedBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
