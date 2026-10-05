//! Port of `Pages/ContainerQueryPage.xaml.cs`: the class of the document
//! `Pages/ContainerQueryPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct ContainerQueryPage {
    base: ContentPage,
}

content_page_class!(ContainerQueryPage);
ferro_class_info!(ContainerQueryPage { new: ContainerQueryPage::new });
xaml_class!(ContainerQueryPage, "/Pages/ContainerQueryPage.xaml");

impl ContainerQueryPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
