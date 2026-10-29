//! Port of `Views/Pages/AutoScrollDuringDragBehaviorView` of the reference sample: the class of `pages/AutoScrollDuringDragBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AutoScrollDuringDragBehaviorView {
    base: UserControl,
}

user_control_class!(AutoScrollDuringDragBehaviorView);
ferro_class_info!(AutoScrollDuringDragBehaviorView {
    new: AutoScrollDuringDragBehaviorView::new
});
xaml_class!(
    AutoScrollDuringDragBehaviorView,
    "/pages/AutoScrollDuringDragBehaviorView.xaml"
);

impl AutoScrollDuringDragBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `AutoScrollDuringDragBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AutoScrollDuringDragBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AutoScrollDuringDragBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Auto Scroll During Drag",
    reference: "Views/Pages/AutoScrollDuringDragBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&AutoScrollDuringDragBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
