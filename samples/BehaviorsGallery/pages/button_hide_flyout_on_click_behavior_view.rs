//! Port of `Views/Pages/ButtonHideFlyoutOnClickBehaviorView` of the reference sample: the class of `pages/ButtonHideFlyoutOnClickBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ButtonHideFlyoutOnClickBehaviorView {
    base: UserControl,
}

user_control_class!(ButtonHideFlyoutOnClickBehaviorView);
ferro_class_info!(ButtonHideFlyoutOnClickBehaviorView { new: ButtonHideFlyoutOnClickBehaviorView::new });
xaml_class!(ButtonHideFlyoutOnClickBehaviorView, "/pages/ButtonHideFlyoutOnClickBehaviorView.xaml");

impl ButtonHideFlyoutOnClickBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ButtonHideFlyoutOnClickBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ButtonHideFlyoutOnClickBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ButtonHideFlyoutOnClickBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ButtonHideFlyoutOnClickBehavior",
    reference: "Views/Pages/ButtonHideFlyoutOnClickBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ButtonHideFlyoutOnClickBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
