//! Port of `Pages/DataValidationPage.xaml.cs`: the class of the document
//! `Pages/DataValidationPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct DataValidationPage {
    base: ContentPage,
}

content_page_class!(DataValidationPage);
ferro_class_info!(DataValidationPage { new: DataValidationPage::new });
xaml_class!(DataValidationPage, "/Pages/DataValidationPage.xaml");

impl DataValidationPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
