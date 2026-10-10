//! Port of `Pages/DrawingPage.xaml.cs`: the class of the document
//! `Pages/DrawingPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct DrawingPage {
    base: UserControl,
}

user_control_class!(DrawingPage);
ferro_class_info!(DrawingPage { new: DrawingPage::new });
xaml_class!(DrawingPage, "/Pages/DrawingPage.xaml");

impl DrawingPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
