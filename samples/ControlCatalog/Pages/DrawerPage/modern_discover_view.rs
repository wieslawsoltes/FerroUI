//! Port of `Pages/DrawerPage/ModernDiscoverView.xaml.cs`: the class of the document
//! `Pages/DrawerPage/ModernDiscoverView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct ModernDiscoverView {
    base: UserControl,
}

user_control_class!(ModernDiscoverView);
ferro_class_info!(ModernDiscoverView { new: ModernDiscoverView::new });
xaml_class!(ModernDiscoverView, "/Pages/DrawerPage/ModernDiscoverView.xaml");

impl ModernDiscoverView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
