//! Port of `Pages/DrawerPage/ModernMyTripsView.xaml.cs`: the class of the document
//! `Pages/DrawerPage/ModernMyTripsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct ModernMyTripsView {
    base: UserControl,
}

user_control_class!(ModernMyTripsView);
ferro_class_info!(ModernMyTripsView { new: ModernMyTripsView::new });
xaml_class!(ModernMyTripsView, "/Pages/DrawerPage/ModernMyTripsView.xaml");

impl ModernMyTripsView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
