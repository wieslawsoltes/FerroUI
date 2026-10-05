//! Port of `Pages/PipsPager/PipsPagerCarouselPage.xaml.cs`: the class of the document
//! `Pages/PipsPager/PipsPagerCarouselPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct PipsPagerCarouselPage {
    base: UserControl,
}

user_control_class!(PipsPagerCarouselPage);
ferro_class_info!(PipsPagerCarouselPage { new: PipsPagerCarouselPage::new });
xaml_class!(PipsPagerCarouselPage, "/Pages/PipsPager/PipsPagerCarouselPage.xaml");

impl PipsPagerCarouselPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
