//! Port of `Views/Pages/AnimateOnAttachedBehaviorView` of the reference sample: the class of `pages/AnimateOnAttachedBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AnimateOnAttachedBehaviorView {
    base: UserControl,
}

user_control_class!(AnimateOnAttachedBehaviorView);
ferro_class_info!(AnimateOnAttachedBehaviorView { new: AnimateOnAttachedBehaviorView::new });
xaml_class!(AnimateOnAttachedBehaviorView, "/pages/AnimateOnAttachedBehaviorView.xaml");

impl AnimateOnAttachedBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `AnimateOnAttachedBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AnimateOnAttachedBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AnimateOnAttachedBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "AnimateOnAttachedBehavior",
    reference: "Views/Pages/AnimateOnAttachedBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&AnimateOnAttachedBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
