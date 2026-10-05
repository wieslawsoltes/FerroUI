//! Port of `Pages/NavigationPage/LAvenirProfileView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/LAvenirProfileView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct LAvenirProfileView {
    base: UserControl,
}

user_control_class!(LAvenirProfileView);
ferro_class_info!(LAvenirProfileView { new: LAvenirProfileView::new });
xaml_class!(LAvenirProfileView, "/Pages/NavigationPage/LAvenirProfileView.xaml");

impl LAvenirProfileView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
