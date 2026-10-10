//! Port of `Pages/CustomAnimatorPage.xaml.cs`: the class of the document
//! `Pages/CustomAnimatorPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct CustomAnimatorPage {
    base: UserControl,
}

user_control_class!(CustomAnimatorPage);
ferro_class_info!(CustomAnimatorPage { new: CustomAnimatorPage::new });
xaml_class!(CustomAnimatorPage, "/Pages/CustomAnimatorPage.xaml");

impl CustomAnimatorPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
