//! Port of `Views/Pages/TextBoxValidationBehaviorView` of the reference sample: the class of `pages/TextBoxValidationBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct TextBoxValidationBehaviorView {
    base: UserControl,
}

user_control_class!(TextBoxValidationBehaviorView);
ferro_class_info!(TextBoxValidationBehaviorView { new: TextBoxValidationBehaviorView::new });
xaml_class!(TextBoxValidationBehaviorView, "/pages/TextBoxValidationBehaviorView.xaml");

impl TextBoxValidationBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `TextBoxValidationBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    TextBoxValidationBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[TextBoxValidationBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "TextBoxValidationBehavior",
    reference: "Views/Pages/TextBoxValidationBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&TextBoxValidationBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
