//! Port of `Views/Pages/PropertyValidationBehaviorView` of the reference sample: the class of
//! `pages/PropertyValidationBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct PropertyValidationBehaviorView {
    base: UserControl,
}

user_control_class!(PropertyValidationBehaviorView);
ferro_class_info!(PropertyValidationBehaviorView {
    new: PropertyValidationBehaviorView::new
});
xaml_class!(
    PropertyValidationBehaviorView,
    "/pages/PropertyValidationBehaviorView.xaml"
);

impl PropertyValidationBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `PropertyValidationBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    PropertyValidationBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[PropertyValidationBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "PropertyValidationBehavior",
    reference: "Views/Pages/PropertyValidationBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&PropertyValidationBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
