//! Port of `Views/Pages/ScreenReaderAnnounceActionView` of the reference sample: the class of `pages/ScreenReaderAnnounceActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ScreenReaderAnnounceActionView {
    base: UserControl,
}

user_control_class!(ScreenReaderAnnounceActionView);
ferro_class_info!(ScreenReaderAnnounceActionView { new: ScreenReaderAnnounceActionView::new });
xaml_class!(ScreenReaderAnnounceActionView, "/pages/ScreenReaderAnnounceActionView.xaml");

impl ScreenReaderAnnounceActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ScreenReaderAnnounceActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ScreenReaderAnnounceActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ScreenReaderAnnounceActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ScreenReaderAnnounceAction",
    reference: "Views/Pages/ScreenReaderAnnounceActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&ScreenReaderAnnounceActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
