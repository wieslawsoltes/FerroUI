//! Port of `Views/Pages/GestureTriggersView` of the reference sample: the class of `pages/GestureTriggersView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct GestureTriggersView {
    base: UserControl,
}

user_control_class!(GestureTriggersView);
ferro_class_info!(GestureTriggersView { new: GestureTriggersView::new });
xaml_class!(GestureTriggersView, "/pages/GestureTriggersView.xaml");

impl GestureTriggersView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `GestureTriggersView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    GestureTriggersView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[GestureTriggersView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Gesture Triggers",
    reference: "Views/Pages/GestureTriggersView",
    create: Some(create),
    types: TYPES,
    classes: &[&GestureTriggersView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
