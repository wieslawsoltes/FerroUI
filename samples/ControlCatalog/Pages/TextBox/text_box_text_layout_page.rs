//! Port of `Pages/TextBox/TextBoxTextLayoutPage.xaml.cs`: the class of the document
//! `Pages/TextBox/TextBoxTextLayoutPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct TextBoxTextLayoutPage {
    base: UserControl,
}

user_control_class!(TextBoxTextLayoutPage);
ferro_class_info!(TextBoxTextLayoutPage { new: TextBoxTextLayoutPage::new });
xaml_class!(TextBoxTextLayoutPage, "/Pages/TextBox/TextBoxTextLayoutPage.xaml");

impl TextBoxTextLayoutPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
