//! Port of `Views/Pages/BindPointerOverBehaviorView` of the reference sample: the class of `pages/BindPointerOverBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct BindPointerOverBehaviorView {
    base: UserControl,
}

user_control_class!(BindPointerOverBehaviorView);
ferro_class_info!(BindPointerOverBehaviorView { new: BindPointerOverBehaviorView::new });
xaml_class!(BindPointerOverBehaviorView, "/pages/BindPointerOverBehaviorView.xaml");

impl BindPointerOverBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `BindPointerOverBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    BindPointerOverBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[BindPointerOverBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "BindPointerOverBehavior",
    reference: "Views/Pages/BindPointerOverBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&BindPointerOverBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
