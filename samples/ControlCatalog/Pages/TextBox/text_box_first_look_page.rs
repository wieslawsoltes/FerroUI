//! Port of `Pages/TextBox/TextBoxFirstLookPage.xaml.cs`: the class of the document
//! `Pages/TextBox/TextBoxFirstLookPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct TextBoxFirstLookPage {
    base: UserControl,
}

user_control_class!(TextBoxFirstLookPage);
ferro_class_info!(TextBoxFirstLookPage { new: TextBoxFirstLookPage::new });
xaml_class!(TextBoxFirstLookPage, "/Pages/TextBox/TextBoxFirstLookPage.xaml");

impl TextBoxFirstLookPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
