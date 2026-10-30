//! Port of `Views/Pages/BindingBehaviorView` of the reference sample: the class of `pages/BindingBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct BindingBehaviorView {
    base: UserControl,
}

user_control_class!(BindingBehaviorView);
ferro_class_info!(BindingBehaviorView { new: BindingBehaviorView::new });
xaml_class!(BindingBehaviorView, "/pages/BindingBehaviorView.xaml");

impl BindingBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `BindingBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    BindingBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[BindingBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "BindingBehavior",
    reference: "Views/Pages/BindingBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&BindingBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
