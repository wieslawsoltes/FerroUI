//! Port of `Pages/ViewboxPage.xaml.cs`: the class of the document
//! `Pages/ViewboxPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct ViewboxPage {
    base: ContentPage,
}

content_page_class!(ViewboxPage);
ferro_class_info!(ViewboxPage { new: ViewboxPage::new });
xaml_class!(ViewboxPage, "/Pages/ViewboxPage.xaml");

impl ViewboxPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
