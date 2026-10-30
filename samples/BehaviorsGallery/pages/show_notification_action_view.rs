//! Port of `Views/Pages/ShowNotificationActionView` of the reference sample: the class of `pages/ShowNotificationActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ShowNotificationActionView {
    base: UserControl,
}

user_control_class!(ShowNotificationActionView);
ferro_class_info!(ShowNotificationActionView {
    new: ShowNotificationActionView::new
});
xaml_class!(
    ShowNotificationActionView,
    "/pages/ShowNotificationActionView.xaml"
);

impl ShowNotificationActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ShowNotificationActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ShowNotificationActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ShowNotificationActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ShowNotificationAction",
    reference: "Views/Pages/ShowNotificationActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&ShowNotificationActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
