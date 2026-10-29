//! Port of `Views/Pages/AutoFocusBehaviorView` of the reference sample: the class of `pages/AutoFocusBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AutoFocusBehaviorView {
    base: UserControl,
}

user_control_class!(AutoFocusBehaviorView);
ferro_class_info!(AutoFocusBehaviorView { new: AutoFocusBehaviorView::new });
xaml_class!(AutoFocusBehaviorView, "/pages/AutoFocusBehaviorView.xaml");

impl AutoFocusBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `AutoFocusBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AutoFocusBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AutoFocusBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "AutoFocusBehavior",
    reference: "Views/Pages/AutoFocusBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&AutoFocusBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
