//! Port of `Views/Pages/AdvancedView` of the reference sample: the class of `pages/AdvancedView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AdvancedView {
    base: UserControl,
}

user_control_class!(AdvancedView);
ferro_class_info!(AdvancedView { new: AdvancedView::new });
xaml_class!(AdvancedView, "/pages/AdvancedView.xaml");

impl AdvancedView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `AdvancedView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AdvancedView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AdvancedView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Advanced Behavior",
    reference: "Views/Pages/AdvancedView",
    create: Some(create),
    types: TYPES,
    classes: &[&AdvancedView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
