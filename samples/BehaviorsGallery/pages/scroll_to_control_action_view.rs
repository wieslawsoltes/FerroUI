//! Port of `Views/Pages/ScrollToControlActionView` of the reference sample: the class of `pages/ScrollToControlActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ScrollToControlActionView {
    base: UserControl,
}

user_control_class!(ScrollToControlActionView);
ferro_class_info!(ScrollToControlActionView { new: ScrollToControlActionView::new });
xaml_class!(ScrollToControlActionView, "/pages/ScrollToControlActionView.xaml");

impl ScrollToControlActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ScrollToControlActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ScrollToControlActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ScrollToControlActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ScrollToControlAction",
    reference: "Views/Pages/ScrollToControlActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&ScrollToControlActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
