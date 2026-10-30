//! Port of `Views/Pages/DragBetweenPanelsView` of the reference sample: the class of `pages/DragBetweenPanelsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DragBetweenPanelsView {
    base: UserControl,
}

user_control_class!(DragBetweenPanelsView);
ferro_class_info!(DragBetweenPanelsView {
    new: DragBetweenPanelsView::new
});
xaml_class!(DragBetweenPanelsView, "/pages/DragBetweenPanelsView.xaml");

impl DragBetweenPanelsView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `DragBetweenPanelsView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DragBetweenPanelsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DragBetweenPanelsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Drag Between Panels",
    reference: "Views/Pages/DragBetweenPanelsView",
    create: Some(create),
    types: TYPES,
    classes: &[&DragBetweenPanelsView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
