//! Port of `Views/Pages/FocusOnAttachedToVisualTreeBehaviorView` of the reference sample: the class of `pages/FocusOnAttachedToVisualTreeBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FocusOnAttachedToVisualTreeBehaviorView {
    base: UserControl,
}

user_control_class!(FocusOnAttachedToVisualTreeBehaviorView);
ferro_class_info!(FocusOnAttachedToVisualTreeBehaviorView { new: FocusOnAttachedToVisualTreeBehaviorView::new });
xaml_class!(FocusOnAttachedToVisualTreeBehaviorView, "/pages/FocusOnAttachedToVisualTreeBehaviorView.xaml");

impl FocusOnAttachedToVisualTreeBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `FocusOnAttachedToVisualTreeBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FocusOnAttachedToVisualTreeBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FocusOnAttachedToVisualTreeBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "FocusOnAttachedToVisualTreeBehavior",
    reference: "Views/Pages/FocusOnAttachedToVisualTreeBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&FocusOnAttachedToVisualTreeBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
