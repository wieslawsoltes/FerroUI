//! Port of `Pages/ResizePatternPage.xaml.cs`: the class of the document
//! `Pages/ResizePatternPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct ResizePatternPage {
    base: UserControl,
}

user_control_class!(ResizePatternPage);
ferro_class_info!(ResizePatternPage { new: ResizePatternPage::new });
xaml_class!(ResizePatternPage, "/Pages/ResizePatternPage.xaml");

impl ResizePatternPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
