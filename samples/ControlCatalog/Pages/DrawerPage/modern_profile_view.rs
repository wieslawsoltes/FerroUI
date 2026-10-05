//! Port of `Pages/DrawerPage/ModernProfileView.xaml.cs`: the class of the document
//! `Pages/DrawerPage/ModernProfileView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct ModernProfileView {
    base: UserControl,
}

user_control_class!(ModernProfileView);
ferro_class_info!(ModernProfileView { new: ModernProfileView::new });
xaml_class!(ModernProfileView, "/Pages/DrawerPage/ModernProfileView.xaml");

impl ModernProfileView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
