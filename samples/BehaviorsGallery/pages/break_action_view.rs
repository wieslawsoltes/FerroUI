//! Port of `Views/Pages/BreakActionView` of the reference sample: the class of `pages/BreakActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct BreakActionView {
    base: UserControl,
}

user_control_class!(BreakActionView);
ferro_class_info!(BreakActionView { new: BreakActionView::new });
xaml_class!(BreakActionView, "/pages/BreakActionView.xaml");

impl BreakActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `BreakActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    BreakActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[BreakActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "BreakAction",
    reference: "Views/Pages/BreakActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&BreakActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
