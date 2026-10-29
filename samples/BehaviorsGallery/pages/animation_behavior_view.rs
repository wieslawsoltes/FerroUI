//! Port of `Views/Pages/AnimationBehaviorView` of the reference sample: the class of `pages/AnimationBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AnimationBehaviorView {
    base: UserControl,
}

user_control_class!(AnimationBehaviorView);
ferro_class_info!(AnimationBehaviorView {
    new: AnimationBehaviorView::new
});
xaml_class!(AnimationBehaviorView, "/pages/AnimationBehaviorView.xaml");

impl AnimationBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `AnimationBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AnimationBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AnimationBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Animation Behaviors",
    reference: "Views/Pages/AnimationBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&AnimationBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
