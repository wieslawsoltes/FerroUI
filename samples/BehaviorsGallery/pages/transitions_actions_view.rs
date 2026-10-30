//! Port of `Views/Pages/TransitionsActionsView` of the reference sample: the class of `pages/TransitionsActionsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct TransitionsActionsView {
    base: UserControl,
}

user_control_class!(TransitionsActionsView);
ferro_class_info!(TransitionsActionsView { new: TransitionsActionsView::new });
xaml_class!(TransitionsActionsView, "/pages/TransitionsActionsView.xaml");

impl TransitionsActionsView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `TransitionsActionsView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    TransitionsActionsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[TransitionsActionsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Transitions Actions",
    reference: "Views/Pages/TransitionsActionsView",
    create: Some(create),
    types: TYPES,
    classes: &[&TransitionsActionsView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
