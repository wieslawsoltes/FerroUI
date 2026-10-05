//! Port of `Pages/AcrylicPage.xaml.cs`: the class of the document
//! `Pages/AcrylicPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct AcrylicPage {
    base: ContentPage,
}

content_page_class!(AcrylicPage);
ferro_class_info!(AcrylicPage { new: AcrylicPage::new });
xaml_class!(AcrylicPage, "/Pages/AcrylicPage.xaml");

impl AcrylicPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
