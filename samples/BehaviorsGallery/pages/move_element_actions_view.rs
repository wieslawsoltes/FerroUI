//! Port of `Views/Pages/MoveElementActionsView` of the reference sample: the class of `pages/MoveElementActionsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct MoveElementActionsView {
    base: UserControl,
}

user_control_class!(MoveElementActionsView);
ferro_class_info!(MoveElementActionsView { new: MoveElementActionsView::new });
xaml_class!(MoveElementActionsView, "/pages/MoveElementActionsView.xaml");

impl MoveElementActionsView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `MoveElementActionsView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    MoveElementActionsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[MoveElementActionsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "MoveElementToPanelAction",
    reference: "Views/Pages/MoveElementActionsView",
    create: Some(create),
    types: TYPES,
    classes: &[&MoveElementActionsView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
