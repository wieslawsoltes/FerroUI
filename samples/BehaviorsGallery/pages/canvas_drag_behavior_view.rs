//! Port of `Views/Pages/CanvasDragBehaviorView` of the reference sample: the class of `pages/CanvasDragBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct CanvasDragBehaviorView {
    base: UserControl,
}

user_control_class!(CanvasDragBehaviorView);
ferro_class_info!(CanvasDragBehaviorView {
    new: CanvasDragBehaviorView::new
});
xaml_class!(CanvasDragBehaviorView, "/pages/CanvasDragBehaviorView.xaml");

impl CanvasDragBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `CanvasDragBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    CanvasDragBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[CanvasDragBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Canvas Drag Behavior",
    reference: "Views/Pages/CanvasDragBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&CanvasDragBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
