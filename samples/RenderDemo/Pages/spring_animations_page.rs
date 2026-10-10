//! Port of `Pages/SpringAnimationsPage.xaml.cs`: the class of the document
//! `Pages/SpringAnimationsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct SpringAnimationsPage {
    base: UserControl,
}

user_control_class!(SpringAnimationsPage);
ferro_class_info!(SpringAnimationsPage { new: SpringAnimationsPage::new });
xaml_class!(SpringAnimationsPage, "/Pages/SpringAnimationsPage.xaml");

impl SpringAnimationsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
