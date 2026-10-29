//! Port of `Views/Pages/ConditionalBehaviorView` of the reference sample: the class of `pages/ConditionalBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ConditionalBehaviorView {
    base: UserControl,
}

user_control_class!(ConditionalBehaviorView);
ferro_class_info!(ConditionalBehaviorView { new: ConditionalBehaviorView::new });
xaml_class!(ConditionalBehaviorView, "/pages/ConditionalBehaviorView.xaml");

impl ConditionalBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ConditionalBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ConditionalBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ConditionalBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ConditionalBehavior",
    reference: "Views/Pages/ConditionalBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ConditionalBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
