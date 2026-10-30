//! Port of `Views/Pages/FocusSelectedItemBehaviorView` of the reference sample: the class of `pages/FocusSelectedItemBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FocusSelectedItemBehaviorView {
    base: UserControl,
}

user_control_class!(FocusSelectedItemBehaviorView);
ferro_class_info!(FocusSelectedItemBehaviorView { new: FocusSelectedItemBehaviorView::new });
xaml_class!(FocusSelectedItemBehaviorView, "/pages/FocusSelectedItemBehaviorView.xaml");

impl FocusSelectedItemBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `FocusSelectedItemBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FocusSelectedItemBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FocusSelectedItemBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "FocusSelectedItemBehavior",
    reference: "Views/Pages/FocusSelectedItemBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&FocusSelectedItemBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
