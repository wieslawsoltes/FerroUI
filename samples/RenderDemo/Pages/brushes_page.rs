//! Port of `Pages/BrushesPage.xaml.cs`: the class of the document
//! `Pages/BrushesPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct BrushesPage {
    base: UserControl,
}

user_control_class!(BrushesPage);
ferro_class_info!(BrushesPage { new: BrushesPage::new });
xaml_class!(BrushesPage, "/Pages/BrushesPage.xaml");

impl BrushesPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
