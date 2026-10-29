//! Port of `Views/Pages/FadeInBehaviorView` of the reference sample: the class of `pages/FadeInBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FadeInBehaviorView {
    base: UserControl,
}

user_control_class!(FadeInBehaviorView);
ferro_class_info!(FadeInBehaviorView { new: FadeInBehaviorView::new });
xaml_class!(FadeInBehaviorView, "/pages/FadeInBehaviorView.xaml");

impl FadeInBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `FadeInBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FadeInBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FadeInBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "FadeInBehavior",
    reference: "Views/Pages/FadeInBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&FadeInBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
