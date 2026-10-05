//! Port of `Pages/RadioButtonPage.xaml.cs`: the class of the document
//! `Pages/RadioButtonPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct RadioButtonPage {
    base: ContentPage,
}

content_page_class!(RadioButtonPage);
ferro_class_info!(RadioButtonPage { new: RadioButtonPage::new });
xaml_class!(RadioButtonPage, "/Pages/RadioButtonPage.xaml");

impl RadioButtonPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
