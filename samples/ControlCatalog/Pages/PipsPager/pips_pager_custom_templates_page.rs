//! Port of `Pages/PipsPager/PipsPagerCustomTemplatesPage.xaml.cs`: the class of the document
//! `Pages/PipsPager/PipsPagerCustomTemplatesPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct PipsPagerCustomTemplatesPage {
    base: UserControl,
}

user_control_class!(PipsPagerCustomTemplatesPage);
ferro_class_info!(PipsPagerCustomTemplatesPage { new: PipsPagerCustomTemplatesPage::new });
xaml_class!(PipsPagerCustomTemplatesPage, "/Pages/PipsPager/PipsPagerCustomTemplatesPage.xaml");

impl PipsPagerCustomTemplatesPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
