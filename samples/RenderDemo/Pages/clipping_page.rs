//! Port of `Pages/ClippingPage.xaml.cs`: the class of the document
//! `Pages/ClippingPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct ClippingPage {
    base: UserControl,
}

user_control_class!(ClippingPage);
ferro_class_info!(ClippingPage { new: ClippingPage::new });
xaml_class!(ClippingPage, "/Pages/ClippingPage.xaml");

impl ClippingPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
