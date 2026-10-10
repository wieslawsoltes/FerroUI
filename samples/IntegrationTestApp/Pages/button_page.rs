//! Port of `Pages/ButtonPage.xaml.cs`: the class of the document `Pages/ButtonPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct ButtonPage {
    base: UserControl,
}

user_control_class!(ButtonPage);
ferro_class_info!(ButtonPage { new: ButtonPage::new });
xaml_class!(ButtonPage, "/Pages/ButtonPage.xaml");

impl ButtonPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
