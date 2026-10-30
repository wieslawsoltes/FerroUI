//! Port of `Views/Pages/PointerEventTriggersView` of the reference sample: the class of `pages/PointerEventTriggersView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct PointerEventTriggersView {
    base: UserControl,
}

user_control_class!(PointerEventTriggersView);
ferro_class_info!(PointerEventTriggersView {
    new: PointerEventTriggersView::new
});
xaml_class!(
    PointerEventTriggersView,
    "/pages/PointerEventTriggersView.xaml"
);

impl PointerEventTriggersView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `PointerEventTriggersView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    PointerEventTriggersView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[PointerEventTriggersView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Pointer Event Triggers",
    reference: "Views/Pages/PointerEventTriggersView",
    create: Some(create),
    types: TYPES,
    classes: &[&PointerEventTriggersView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
