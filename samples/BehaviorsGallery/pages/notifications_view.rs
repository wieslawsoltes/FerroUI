//! Port of `Views/Pages/NotificationsView` of the reference sample: the class of `pages/NotificationsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct NotificationsView {
    base: UserControl,
}

user_control_class!(NotificationsView);
ferro_class_info!(NotificationsView { new: NotificationsView::new });
xaml_class!(NotificationsView, "/pages/NotificationsView.xaml");

impl NotificationsView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `NotificationsView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    NotificationsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[NotificationsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Notifications",
    reference: "Views/Pages/NotificationsView",
    create: Some(create),
    types: TYPES,
    classes: &[&NotificationsView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
