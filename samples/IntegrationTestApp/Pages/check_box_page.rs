//! Port of `Pages/CheckBoxPage.xaml.cs`: the class of the document `Pages/CheckBoxPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct CheckBoxPage {
    base: UserControl,
}

user_control_class!(CheckBoxPage);
ferro_class_info!(CheckBoxPage { new: CheckBoxPage::new });
xaml_class!(CheckBoxPage, "/Pages/CheckBoxPage.xaml");

impl CheckBoxPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
