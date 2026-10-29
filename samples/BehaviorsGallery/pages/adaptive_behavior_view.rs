//! Port of `Views/Pages/AdaptiveBehaviorView` of the reference sample: the class of `pages/AdaptiveBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AdaptiveBehaviorView {
    base: UserControl,
}

user_control_class!(AdaptiveBehaviorView);
ferro_class_info!(AdaptiveBehaviorView {
    new: AdaptiveBehaviorView::new
});
xaml_class!(AdaptiveBehaviorView, "/pages/AdaptiveBehaviorView.xaml");

impl AdaptiveBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `AdaptiveBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AdaptiveBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AdaptiveBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "AdaptiveBehavior",
    reference: "Views/Pages/AdaptiveBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&AdaptiveBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
