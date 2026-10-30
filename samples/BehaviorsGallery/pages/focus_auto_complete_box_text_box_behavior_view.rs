//! Port of `Views/Pages/FocusAutoCompleteBoxTextBoxBehaviorView` of the reference sample: the class of `pages/FocusAutoCompleteBoxTextBoxBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FocusAutoCompleteBoxTextBoxBehaviorView {
    base: UserControl,
}

user_control_class!(FocusAutoCompleteBoxTextBoxBehaviorView);
ferro_class_info!(FocusAutoCompleteBoxTextBoxBehaviorView { new: FocusAutoCompleteBoxTextBoxBehaviorView::new });
xaml_class!(FocusAutoCompleteBoxTextBoxBehaviorView, "/pages/FocusAutoCompleteBoxTextBoxBehaviorView.xaml");

impl FocusAutoCompleteBoxTextBoxBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `FocusAutoCompleteBoxTextBoxBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FocusAutoCompleteBoxTextBoxBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FocusAutoCompleteBoxTextBoxBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "FocusAutoCompleteBoxTextBoxBehavior",
    reference: "Views/Pages/FocusAutoCompleteBoxTextBoxBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&FocusAutoCompleteBoxTextBoxBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
