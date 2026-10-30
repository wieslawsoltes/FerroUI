//! Port of `Views/Pages/FocusTrapBehaviorView` of the reference sample: the class of `pages/FocusTrapBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FocusTrapBehaviorView {
    base: UserControl,
}

user_control_class!(FocusTrapBehaviorView);
ferro_class_info!(FocusTrapBehaviorView { new: FocusTrapBehaviorView::new });
xaml_class!(FocusTrapBehaviorView, "/pages/FocusTrapBehaviorView.xaml");

impl FocusTrapBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `FocusTrapBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FocusTrapBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FocusTrapBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "FocusTrapBehavior",
    reference: "Views/Pages/FocusTrapBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&FocusTrapBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
