//! Port of `Views/Pages/AutoSelectBehaviorView` of the reference sample: the class of `pages/AutoSelectBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AutoSelectBehaviorView {
    base: UserControl,
}

user_control_class!(AutoSelectBehaviorView);
ferro_class_info!(AutoSelectBehaviorView { new: AutoSelectBehaviorView::new });
xaml_class!(AutoSelectBehaviorView, "/pages/AutoSelectBehaviorView.xaml");

impl AutoSelectBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `AutoSelectBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AutoSelectBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AutoSelectBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "AutoSelectBehavior",
    reference: "Views/Pages/AutoSelectBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&AutoSelectBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
