//! Port of `Views/Pages/AutoCompleteBoxView` of the reference sample: the class of `pages/AutoCompleteBoxView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AutoCompleteBoxView {
    base: UserControl,
}

user_control_class!(AutoCompleteBoxView);
ferro_class_info!(AutoCompleteBoxView { new: AutoCompleteBoxView::new });
xaml_class!(AutoCompleteBoxView, "/pages/AutoCompleteBoxView.xaml");

impl AutoCompleteBoxView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `AutoCompleteBoxView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AutoCompleteBoxView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AutoCompleteBoxView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "AutoCompleteBox",
    reference: "Views/Pages/AutoCompleteBoxView",
    create: Some(create),
    types: TYPES,
    classes: &[&AutoCompleteBoxView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
