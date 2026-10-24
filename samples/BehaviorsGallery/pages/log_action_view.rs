//! Port of `Views/Pages/LogActionView` of the reference sample: the class of `pages/LogActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct LogActionView {
    base: UserControl,
}

user_control_class!(LogActionView);
ferro_class_info!(LogActionView { new: LogActionView::new });
xaml_class!(LogActionView, "/pages/LogActionView.xaml");

impl LogActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `LogActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    LogActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[LogActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "LogAction",
    reference: "Views/Pages/LogActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&LogActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
