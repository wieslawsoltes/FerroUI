//! Port of `Pages/DrawerPage/ModernSettingsView.xaml.cs`: the class of the document
//! `Pages/DrawerPage/ModernSettingsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct ModernSettingsView {
    base: UserControl,
}

user_control_class!(ModernSettingsView);
ferro_class_info!(ModernSettingsView { new: ModernSettingsView::new });
xaml_class!(ModernSettingsView, "/Pages/DrawerPage/ModernSettingsView.xaml");

impl ModernSettingsView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
