//! Port of `Pages/LineBoundsPage.xaml.cs`: the class of the document
//! `Pages/LineBoundsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct LineBoundsPage {
    base: UserControl,
}

user_control_class!(LineBoundsPage);
ferro_class_info!(LineBoundsPage { new: LineBoundsPage::new });
xaml_class!(LineBoundsPage, "/Pages/LineBoundsPage.xaml");

impl LineBoundsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
