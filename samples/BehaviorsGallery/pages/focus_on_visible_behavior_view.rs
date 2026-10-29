//! Port of `Views/Pages/FocusOnVisibleBehaviorView` of the reference sample: the class of `pages/FocusOnVisibleBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FocusOnVisibleBehaviorView {
    base: UserControl,
}

user_control_class!(FocusOnVisibleBehaviorView);
ferro_class_info!(FocusOnVisibleBehaviorView { new: FocusOnVisibleBehaviorView::new });
xaml_class!(FocusOnVisibleBehaviorView, "/pages/FocusOnVisibleBehaviorView.xaml");

impl FocusOnVisibleBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `FocusOnVisibleBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FocusOnVisibleBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FocusOnVisibleBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "FocusOnVisibleBehavior",
    reference: "Views/Pages/FocusOnVisibleBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&FocusOnVisibleBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
