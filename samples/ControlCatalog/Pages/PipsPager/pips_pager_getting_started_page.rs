//! Port of `Pages/PipsPager/PipsPagerGettingStartedPage.xaml.cs`: the class of the document
//! `Pages/PipsPager/PipsPagerGettingStartedPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct PipsPagerGettingStartedPage {
    base: UserControl,
}

user_control_class!(PipsPagerGettingStartedPage);
ferro_class_info!(PipsPagerGettingStartedPage { new: PipsPagerGettingStartedPage::new });
xaml_class!(PipsPagerGettingStartedPage, "/Pages/PipsPager/PipsPagerGettingStartedPage.xaml");

impl PipsPagerGettingStartedPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
