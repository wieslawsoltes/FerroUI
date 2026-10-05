//! Port of `Pages/PipsPager/PipsPagerCustomButtonThemesPage.xaml.cs`: the class of the document
//! `Pages/PipsPager/PipsPagerCustomButtonThemesPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct PipsPagerCustomButtonThemesPage {
    base: UserControl,
}

user_control_class!(PipsPagerCustomButtonThemesPage);
ferro_class_info!(PipsPagerCustomButtonThemesPage { new: PipsPagerCustomButtonThemesPage::new });
xaml_class!(PipsPagerCustomButtonThemesPage, "/Pages/PipsPager/PipsPagerCustomButtonThemesPage.xaml");

impl PipsPagerCustomButtonThemesPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
