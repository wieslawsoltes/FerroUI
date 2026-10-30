//! Port of `Views/Pages/ViewModelPropertyActionsView` of the reference sample: the class of `pages/ViewModelPropertyActionsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ViewModelPropertyActionsView {
    base: UserControl,
}

user_control_class!(ViewModelPropertyActionsView);
ferro_class_info!(ViewModelPropertyActionsView { new: ViewModelPropertyActionsView::new });
xaml_class!(ViewModelPropertyActionsView, "/pages/ViewModelPropertyActionsView.xaml");

impl ViewModelPropertyActionsView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ViewModelPropertyActionsView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ViewModelPropertyActionsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ViewModelPropertyActionsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ViewModel Property Actions",
    reference: "Views/Pages/ViewModelPropertyActionsView",
    create: Some(create),
    types: TYPES,
    classes: &[&ViewModelPropertyActionsView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
