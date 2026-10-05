//! Port of `Pages/FocusPage.xaml.cs`: the class of the document
//! `Pages/FocusPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct FocusPage {
    base: ContentPage,
}

content_page_class!(FocusPage);
ferro_class_info!(FocusPage { new: FocusPage::new });
xaml_class!(FocusPage, "/Pages/FocusPage.xaml");

impl FocusPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
