//! Port of `Views/Pages/ButtonHideFlyoutBehaviorView` of the reference sample: the class of `pages/ButtonHideFlyoutBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ButtonHideFlyoutBehaviorView {
    base: UserControl,
}

user_control_class!(ButtonHideFlyoutBehaviorView);
ferro_class_info!(ButtonHideFlyoutBehaviorView { new: ButtonHideFlyoutBehaviorView::new });
xaml_class!(ButtonHideFlyoutBehaviorView, "/pages/ButtonHideFlyoutBehaviorView.xaml");

impl ButtonHideFlyoutBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ButtonHideFlyoutBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ButtonHideFlyoutBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ButtonHideFlyoutBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ButtonHideFlyoutBehavior",
    reference: "Views/Pages/ButtonHideFlyoutBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ButtonHideFlyoutBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
