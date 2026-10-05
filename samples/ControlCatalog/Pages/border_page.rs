//! Port of `Pages/BorderPage.xaml.cs`: the class of the document
//! `Pages/BorderPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct BorderPage {
    base: ContentPage,
}

content_page_class!(BorderPage);
ferro_class_info!(BorderPage { new: BorderPage::new });
xaml_class!(BorderPage, "/Pages/BorderPage.xaml");

impl BorderPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
