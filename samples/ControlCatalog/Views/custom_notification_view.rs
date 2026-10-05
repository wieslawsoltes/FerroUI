//! Port of `Views/CustomNotificationView.xaml.cs`: the class of the
//! document `Views/CustomNotificationView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct CustomNotificationView {
    base: UserControl,
}

user_control_class!(CustomNotificationView);
ferro_class_info!(CustomNotificationView { new: CustomNotificationView::new });
xaml_class!(CustomNotificationView, "/Views/CustomNotificationView.xaml");

impl CustomNotificationView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
