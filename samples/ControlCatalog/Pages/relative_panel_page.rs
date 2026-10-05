//! Port of `Pages/RelativePanelPage.xaml.cs`: the class of the document
//! `Pages/RelativePanelPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct RelativePanelPage {
    base: ContentPage,
}

content_page_class!(RelativePanelPage);
ferro_class_info!(RelativePanelPage { new: RelativePanelPage::new });
xaml_class!(RelativePanelPage, "/Pages/RelativePanelPage.xaml");

impl RelativePanelPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
