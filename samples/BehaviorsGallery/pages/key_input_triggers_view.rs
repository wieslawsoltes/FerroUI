//! Port of `Views/Pages/KeyInputTriggersView` of the reference sample: the class of `pages/KeyInputTriggersView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct KeyInputTriggersView {
    base: UserControl,
}

user_control_class!(KeyInputTriggersView);
ferro_class_info!(KeyInputTriggersView { new: KeyInputTriggersView::new });
xaml_class!(KeyInputTriggersView, "/pages/KeyInputTriggersView.xaml");

impl KeyInputTriggersView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `KeyInputTriggersView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    KeyInputTriggersView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[KeyInputTriggersView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Key Input Triggers",
    reference: "Views/Pages/KeyInputTriggersView",
    create: Some(create),
    types: TYPES,
    classes: &[&KeyInputTriggersView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
