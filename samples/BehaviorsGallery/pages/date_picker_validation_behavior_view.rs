//! Port of `Views/Pages/DatePickerValidationBehaviorView` of the reference sample: the class of `pages/DatePickerValidationBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DatePickerValidationBehaviorView {
    base: UserControl,
}

user_control_class!(DatePickerValidationBehaviorView);
ferro_class_info!(DatePickerValidationBehaviorView { new: DatePickerValidationBehaviorView::new });
xaml_class!(DatePickerValidationBehaviorView, "/pages/DatePickerValidationBehaviorView.xaml");

impl DatePickerValidationBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `DatePickerValidationBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DatePickerValidationBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DatePickerValidationBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "DatePickerValidationBehavior",
    reference: "Views/Pages/DatePickerValidationBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&DatePickerValidationBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
