//! Port of `Views/Pages/AutomationView` of the reference sample: the class of `pages/AutomationView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AutomationView {
    base: UserControl,
}

user_control_class!(AutomationView);
ferro_class_info!(AutomationView { new: AutomationView::new });
xaml_class!(AutomationView, "/pages/AutomationView.xaml");

impl AutomationView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `AutomationView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AutomationView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AutomationView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Automation",
    reference: "Views/Pages/AutomationView",
    create: Some(create),
    types: TYPES,
    classes: &[&AutomationView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
