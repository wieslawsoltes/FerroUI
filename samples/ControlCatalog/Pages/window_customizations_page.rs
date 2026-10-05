//! Port of `Pages/WindowCustomizationsPage.xaml.cs`: the class of the document
//! `Pages/WindowCustomizationsPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct WindowCustomizationsPage {
    base: ContentPage,
}

content_page_class!(WindowCustomizationsPage);
ferro_class_info!(WindowCustomizationsPage { new: WindowCustomizationsPage::new });
xaml_class!(WindowCustomizationsPage, "/Pages/WindowCustomizationsPage.xaml");

impl WindowCustomizationsPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
