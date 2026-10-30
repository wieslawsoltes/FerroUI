//! Port of `Views/Pages/VisualDebugBehaviorView` of the reference sample: the class of `pages/VisualDebugBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct VisualDebugBehaviorView {
    base: UserControl,
}

user_control_class!(VisualDebugBehaviorView);
ferro_class_info!(VisualDebugBehaviorView { new: VisualDebugBehaviorView::new });
xaml_class!(VisualDebugBehaviorView, "/pages/VisualDebugBehaviorView.xaml");

impl VisualDebugBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `VisualDebugBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    VisualDebugBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[VisualDebugBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "VisualDebugBehavior",
    reference: "Views/Pages/VisualDebugBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&VisualDebugBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
