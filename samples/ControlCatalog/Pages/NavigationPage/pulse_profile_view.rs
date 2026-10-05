//! Port of `Pages/NavigationPage/PulseProfileView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/PulseProfileView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct PulseProfileView {
    base: UserControl,
}

user_control_class!(PulseProfileView);
ferro_class_info!(PulseProfileView { new: PulseProfileView::new });
xaml_class!(PulseProfileView, "/Pages/NavigationPage/PulseProfileView.xaml");

impl PulseProfileView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
