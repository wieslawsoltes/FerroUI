//! Port of `Pages/ToggleSwitchPage.xaml.cs`: the class of the document
//! `Pages/ToggleSwitchPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct ToggleSwitchPage {
    base: ContentPage,
}

content_page_class!(ToggleSwitchPage);
ferro_class_info!(ToggleSwitchPage { new: ToggleSwitchPage::new });
xaml_class!(ToggleSwitchPage, "/Pages/ToggleSwitchPage.xaml");

impl ToggleSwitchPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
