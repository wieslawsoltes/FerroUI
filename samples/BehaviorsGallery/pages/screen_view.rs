//! Port of `Views/Pages/ScreenView` of the reference sample: the class of `pages/ScreenView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ScreenView {
    base: UserControl,
}

user_control_class!(ScreenView);
ferro_class_info!(ScreenView { new: ScreenView::new });
xaml_class!(ScreenView, "/pages/ScreenView.xaml");

impl ScreenView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ScreenView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ScreenView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ScreenView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Screens",
    reference: "Views/Pages/ScreenView",
    create: Some(create),
    types: TYPES,
    classes: &[&ScreenView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
