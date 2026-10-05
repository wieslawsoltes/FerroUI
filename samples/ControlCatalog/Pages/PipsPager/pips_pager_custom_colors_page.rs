//! Port of `Pages/PipsPager/PipsPagerCustomColorsPage.xaml.cs`: the class of the document
//! `Pages/PipsPager/PipsPagerCustomColorsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct PipsPagerCustomColorsPage {
    base: UserControl,
}

user_control_class!(PipsPagerCustomColorsPage);
ferro_class_info!(PipsPagerCustomColorsPage { new: PipsPagerCustomColorsPage::new });
xaml_class!(PipsPagerCustomColorsPage, "/Pages/PipsPager/PipsPagerCustomColorsPage.xaml");

impl PipsPagerCustomColorsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
