//! Port of `Pages/AcceleratorPage.xaml.cs`: the class of the document
//! `Pages/AcceleratorPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct AcceleratorPage {
    base: ContentPage,
}

content_page_class!(AcceleratorPage);
ferro_class_info!(AcceleratorPage { new: AcceleratorPage::new });
xaml_class!(AcceleratorPage, "/Pages/AcceleratorPage.xaml");

impl AcceleratorPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
