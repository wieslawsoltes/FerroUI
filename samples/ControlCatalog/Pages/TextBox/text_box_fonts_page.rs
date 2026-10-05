//! Port of `Pages/TextBox/TextBoxFontsPage.xaml.cs`: the class of the document
//! `Pages/TextBox/TextBoxFontsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct TextBoxFontsPage {
    base: UserControl,
}

user_control_class!(TextBoxFontsPage);
ferro_class_info!(TextBoxFontsPage { new: TextBoxFontsPage::new });
xaml_class!(TextBoxFontsPage, "/Pages/TextBox/TextBoxFontsPage.xaml");

impl TextBoxFontsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
