//! Port of `Views/Pages/ComboBoxValidationBehaviorView` of the reference sample: the class of `pages/ComboBoxValidationBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ComboBoxValidationBehaviorView {
    base: UserControl,
}

user_control_class!(ComboBoxValidationBehaviorView);
ferro_class_info!(ComboBoxValidationBehaviorView { new: ComboBoxValidationBehaviorView::new });
xaml_class!(ComboBoxValidationBehaviorView, "/pages/ComboBoxValidationBehaviorView.xaml");

impl ComboBoxValidationBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ComboBoxValidationBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ComboBoxValidationBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ComboBoxValidationBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ComboBoxValidationBehavior",
    reference: "Views/Pages/ComboBoxValidationBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ComboBoxValidationBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
