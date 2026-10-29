//! Port of `Views/Pages/ToolTipHelpersView` of the reference sample: the class of `pages/ToolTipHelpersView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ToolTipHelpersView {
    base: UserControl,
}

user_control_class!(ToolTipHelpersView);
ferro_class_info!(ToolTipHelpersView { new: ToolTipHelpersView::new });
xaml_class!(ToolTipHelpersView, "/pages/ToolTipHelpersView.xaml");

impl ToolTipHelpersView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ToolTipHelpersView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ToolTipHelpersView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ToolTipHelpersView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ToolTip Helpers",
    reference: "Views/Pages/ToolTipHelpersView",
    create: Some(create),
    types: TYPES,
    classes: &[&ToolTipHelpersView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
