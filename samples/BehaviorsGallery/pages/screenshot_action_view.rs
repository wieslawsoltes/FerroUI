//! Port of `Views/Pages/ScreenshotActionView` of the reference sample: the class of `pages/ScreenshotActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ScreenshotActionView {
    base: UserControl,
}

user_control_class!(ScreenshotActionView);
ferro_class_info!(ScreenshotActionView { new: ScreenshotActionView::new });
xaml_class!(ScreenshotActionView, "/pages/ScreenshotActionView.xaml");

impl ScreenshotActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ScreenshotActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ScreenshotActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ScreenshotActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ScreenshotAction",
    reference: "Views/Pages/ScreenshotActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&ScreenshotActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
