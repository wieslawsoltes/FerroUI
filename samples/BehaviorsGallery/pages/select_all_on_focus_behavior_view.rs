//! Port of `Views/Pages/SelectAllOnFocusBehaviorView` of the reference sample: the class of `pages/SelectAllOnFocusBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct SelectAllOnFocusBehaviorView {
    base: UserControl,
}

user_control_class!(SelectAllOnFocusBehaviorView);
ferro_class_info!(SelectAllOnFocusBehaviorView { new: SelectAllOnFocusBehaviorView::new });
xaml_class!(SelectAllOnFocusBehaviorView, "/pages/SelectAllOnFocusBehaviorView.xaml");

impl SelectAllOnFocusBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `SelectAllOnFocusBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    SelectAllOnFocusBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[SelectAllOnFocusBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "SelectAllOnFocusBehavior",
    reference: "Views/Pages/SelectAllOnFocusBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&SelectAllOnFocusBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
