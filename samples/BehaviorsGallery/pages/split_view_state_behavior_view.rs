//! Port of `Views/Pages/SplitViewStateBehaviorView` of the reference sample: the class of `pages/SplitViewStateBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct SplitViewStateBehaviorView {
    base: UserControl,
}

user_control_class!(SplitViewStateBehaviorView);
ferro_class_info!(SplitViewStateBehaviorView { new: SplitViewStateBehaviorView::new });
xaml_class!(SplitViewStateBehaviorView, "/pages/SplitViewStateBehaviorView.xaml");

impl SplitViewStateBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `SplitViewStateBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    SplitViewStateBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[SplitViewStateBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "SplitView",
    reference: "Views/Pages/SplitViewStateBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&SplitViewStateBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
