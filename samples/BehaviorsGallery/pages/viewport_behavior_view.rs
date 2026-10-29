//! Port of `Views/Pages/ViewportBehaviorView` of the reference sample: the class of `pages/ViewportBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ViewportBehaviorView {
    base: UserControl,
}

user_control_class!(ViewportBehaviorView);
ferro_class_info!(ViewportBehaviorView { new: ViewportBehaviorView::new });
xaml_class!(ViewportBehaviorView, "/pages/ViewportBehaviorView.xaml");

impl ViewportBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ViewportBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ViewportBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ViewportBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ViewportBehavior",
    reference: "Views/Pages/ViewportBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ViewportBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
