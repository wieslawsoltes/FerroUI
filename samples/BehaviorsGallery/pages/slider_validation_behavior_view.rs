//! Port of `Views/Pages/SliderValidationBehaviorView` of the reference sample: the class of `pages/SliderValidationBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct SliderValidationBehaviorView {
    base: UserControl,
}

user_control_class!(SliderValidationBehaviorView);
ferro_class_info!(SliderValidationBehaviorView { new: SliderValidationBehaviorView::new });
xaml_class!(SliderValidationBehaviorView, "/pages/SliderValidationBehaviorView.xaml");

impl SliderValidationBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `SliderValidationBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    SliderValidationBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[SliderValidationBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "SliderValidationBehavior",
    reference: "Views/Pages/SliderValidationBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&SliderValidationBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
