//! Port of `Views/Pages/ShowHideFlyoutActionView` of the reference sample: the class of `pages/ShowHideFlyoutActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ShowHideFlyoutActionView {
    base: UserControl,
}

user_control_class!(ShowHideFlyoutActionView);
ferro_class_info!(ShowHideFlyoutActionView {
    new: ShowHideFlyoutActionView::new
});
xaml_class!(
    ShowHideFlyoutActionView,
    "/pages/ShowHideFlyoutActionView.xaml"
);

impl ShowHideFlyoutActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ShowHideFlyoutActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ShowHideFlyoutActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ShowHideFlyoutActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ShowHideFlyoutAction",
    reference: "Views/Pages/ShowHideFlyoutActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&ShowHideFlyoutActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
