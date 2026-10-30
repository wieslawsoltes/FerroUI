//! Port of `Views/Pages/ShowHidePopupActionView` of the reference sample: the class of `pages/ShowHidePopupActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ShowHidePopupActionView {
    base: UserControl,
}

user_control_class!(ShowHidePopupActionView);
ferro_class_info!(ShowHidePopupActionView {
    new: ShowHidePopupActionView::new
});
xaml_class!(
    ShowHidePopupActionView,
    "/pages/ShowHidePopupActionView.xaml"
);

impl ShowHidePopupActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ShowHidePopupActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ShowHidePopupActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ShowHidePopupActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ShowHidePopupAction",
    reference: "Views/Pages/ShowHidePopupActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&ShowHidePopupActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
