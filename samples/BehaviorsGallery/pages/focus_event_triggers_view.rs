//! Port of `Views/Pages/FocusEventTriggersView` of the reference sample: the class of `pages/FocusEventTriggersView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FocusEventTriggersView {
    base: UserControl,
}

user_control_class!(FocusEventTriggersView);
ferro_class_info!(FocusEventTriggersView { new: FocusEventTriggersView::new });
xaml_class!(FocusEventTriggersView, "/pages/FocusEventTriggersView.xaml");

impl FocusEventTriggersView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `FocusEventTriggersView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FocusEventTriggersView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FocusEventTriggersView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Focus Event Triggers",
    reference: "Views/Pages/FocusEventTriggersView",
    create: Some(create),
    types: TYPES,
    classes: &[&FocusEventTriggersView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
