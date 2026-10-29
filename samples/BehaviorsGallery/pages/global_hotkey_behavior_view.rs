//! Port of `Views/Pages/GlobalHotkeyBehaviorView` of the reference sample: the class of `pages/GlobalHotkeyBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct GlobalHotkeyBehaviorView {
    base: UserControl,
}

user_control_class!(GlobalHotkeyBehaviorView);
ferro_class_info!(GlobalHotkeyBehaviorView { new: GlobalHotkeyBehaviorView::new });
xaml_class!(GlobalHotkeyBehaviorView, "/pages/GlobalHotkeyBehaviorView.xaml");

impl GlobalHotkeyBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `GlobalHotkeyBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    GlobalHotkeyBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[GlobalHotkeyBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "GlobalHotkeyBehavior",
    reference: "Views/Pages/GlobalHotkeyBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&GlobalHotkeyBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
