//! Port of `Views/Pages/FocusControlBehaviorView` of the reference sample: the class of `pages/FocusControlBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FocusControlBehaviorView {
    base: UserControl,
}

user_control_class!(FocusControlBehaviorView);
ferro_class_info!(FocusControlBehaviorView { new: FocusControlBehaviorView::new });
xaml_class!(FocusControlBehaviorView, "/pages/FocusControlBehaviorView.xaml");

impl FocusControlBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `FocusControlBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FocusControlBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FocusControlBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "FocusControlBehavior",
    reference: "Views/Pages/FocusControlBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&FocusControlBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
