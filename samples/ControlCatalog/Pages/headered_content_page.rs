//! Port of `Pages/HeaderedContentPage.xaml.cs`: the class of the document
//! `Pages/HeaderedContentPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct HeaderedContentPage {
    base: ContentPage,
}

content_page_class!(HeaderedContentPage);
ferro_class_info!(HeaderedContentPage { new: HeaderedContentPage::new });
xaml_class!(HeaderedContentPage, "/Pages/HeaderedContentPage.xaml");

impl HeaderedContentPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
