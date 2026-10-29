//! Port of `Views/Pages/FocusOnPointerMovedBehaviorView` of the reference sample: the class of `pages/FocusOnPointerMovedBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FocusOnPointerMovedBehaviorView {
    base: UserControl,
}

user_control_class!(FocusOnPointerMovedBehaviorView);
ferro_class_info!(FocusOnPointerMovedBehaviorView { new: FocusOnPointerMovedBehaviorView::new });
xaml_class!(FocusOnPointerMovedBehaviorView, "/pages/FocusOnPointerMovedBehaviorView.xaml");

impl FocusOnPointerMovedBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `FocusOnPointerMovedBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FocusOnPointerMovedBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FocusOnPointerMovedBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "FocusOnPointerMovedBehavior",
    reference: "Views/Pages/FocusOnPointerMovedBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&FocusOnPointerMovedBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
