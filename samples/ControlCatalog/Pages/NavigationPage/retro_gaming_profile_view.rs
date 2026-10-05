//! Port of `Pages/NavigationPage/RetroGamingProfileView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/RetroGamingProfileView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct RetroGamingProfileView {
    base: UserControl,
}

user_control_class!(RetroGamingProfileView);
ferro_class_info!(RetroGamingProfileView { new: RetroGamingProfileView::new });
xaml_class!(RetroGamingProfileView, "/Pages/NavigationPage/RetroGamingProfileView.xaml");

impl RetroGamingProfileView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
