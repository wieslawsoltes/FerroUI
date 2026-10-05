//! Port of `Pages/CarouselPage/SanctuaryMainPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/SanctuaryMainPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct SanctuaryMainPage {
    base: UserControl,
}

user_control_class!(SanctuaryMainPage);
ferro_class_info!(SanctuaryMainPage { new: SanctuaryMainPage::new });
xaml_class!(SanctuaryMainPage, "/Pages/CarouselPage/SanctuaryMainPage.xaml");

impl SanctuaryMainPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
