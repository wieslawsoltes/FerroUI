//! Port of `Views/Pages/TabControlNavigationView` of the reference sample: the class of `pages/TabControlNavigationView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct TabControlNavigationView {
    base: UserControl,
}

user_control_class!(TabControlNavigationView);
ferro_class_info!(TabControlNavigationView { new: TabControlNavigationView::new });
xaml_class!(TabControlNavigationView, "/pages/TabControlNavigationView.xaml");

impl TabControlNavigationView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `TabControlNavigationView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    TabControlNavigationView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[TabControlNavigationView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "TabControl Navigation",
    reference: "Views/Pages/TabControlNavigationView",
    create: Some(create),
    types: TYPES,
    classes: &[&TabControlNavigationView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
