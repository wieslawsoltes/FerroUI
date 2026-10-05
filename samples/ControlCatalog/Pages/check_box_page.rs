//! Port of `Pages/CheckBoxPage.xaml.cs`: the class of the document
//! `Pages/CheckBoxPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct CheckBoxPage {
    base: ContentPage,
}

content_page_class!(CheckBoxPage);
ferro_class_info!(CheckBoxPage { new: CheckBoxPage::new });
xaml_class!(CheckBoxPage, "/Pages/CheckBoxPage.xaml");

impl CheckBoxPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
