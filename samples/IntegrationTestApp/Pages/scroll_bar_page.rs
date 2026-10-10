//! Port of `Pages/ScrollBarPage.xaml.cs`: the class of the document `Pages/ScrollBarPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct ScrollBarPage {
    base: UserControl,
}

user_control_class!(ScrollBarPage);
ferro_class_info!(ScrollBarPage { new: ScrollBarPage::new });
xaml_class!(ScrollBarPage, "/Pages/ScrollBarPage.xaml");

impl ScrollBarPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
