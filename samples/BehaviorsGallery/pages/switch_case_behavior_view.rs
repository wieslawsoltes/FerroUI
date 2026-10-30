//! Port of `Views/Pages/SwitchCaseBehaviorView` of the reference sample: the class of `pages/SwitchCaseBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct SwitchCaseBehaviorView {
    base: UserControl,
}

user_control_class!(SwitchCaseBehaviorView);
ferro_class_info!(SwitchCaseBehaviorView { new: SwitchCaseBehaviorView::new });
xaml_class!(SwitchCaseBehaviorView, "/pages/SwitchCaseBehaviorView.xaml");

impl SwitchCaseBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `SwitchCaseBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    SwitchCaseBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[SwitchCaseBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "SwitchCaseBehavior",
    reference: "Views/Pages/SwitchCaseBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&SwitchCaseBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
