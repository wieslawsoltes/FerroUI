//! Port of `Views/Pages/KeyEventTriggersView` of the reference sample: the class of `pages/KeyEventTriggersView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct KeyEventTriggersView {
    base: UserControl,
}

user_control_class!(KeyEventTriggersView);
ferro_class_info!(KeyEventTriggersView { new: KeyEventTriggersView::new });
xaml_class!(KeyEventTriggersView, "/pages/KeyEventTriggersView.xaml");

impl KeyEventTriggersView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `KeyEventTriggersView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    KeyEventTriggersView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[KeyEventTriggersView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Key Event Triggers",
    reference: "Views/Pages/KeyEventTriggersView",
    create: Some(create),
    types: TYPES,
    classes: &[&KeyEventTriggersView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
