//! Port of `Views/Pages/BindTagToVisualRootDataContextBehaviorView` of the reference sample: the class of `pages/BindTagToVisualRootDataContextBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct BindTagToVisualRootDataContextBehaviorView {
    base: UserControl,
}

user_control_class!(BindTagToVisualRootDataContextBehaviorView);
ferro_class_info!(BindTagToVisualRootDataContextBehaviorView { new: BindTagToVisualRootDataContextBehaviorView::new });
xaml_class!(BindTagToVisualRootDataContextBehaviorView, "/pages/BindTagToVisualRootDataContextBehaviorView.xaml");

impl BindTagToVisualRootDataContextBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `BindTagToVisualRootDataContextBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    BindTagToVisualRootDataContextBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[BindTagToVisualRootDataContextBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "BindTagToVisualRootDataContextBehavior",
    reference: "Views/Pages/BindTagToVisualRootDataContextBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&BindTagToVisualRootDataContextBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
