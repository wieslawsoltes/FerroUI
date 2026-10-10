//! Port of `Pages/RadioButtonPage.xaml.cs`: the class of the document
//! `Pages/RadioButtonPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct RadioButtonPage {
    base: UserControl,
}

user_control_class!(RadioButtonPage);
ferro_class_info!(RadioButtonPage { new: RadioButtonPage::new });
xaml_class!(RadioButtonPage, "/Pages/RadioButtonPage.xaml");

impl RadioButtonPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
