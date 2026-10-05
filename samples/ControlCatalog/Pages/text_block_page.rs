//! Port of `Pages/TextBlockPage.xaml.cs`: the class of the document
//! `Pages/TextBlockPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct TextBlockPage {
    base: ContentPage,
}

content_page_class!(TextBlockPage);
ferro_class_info!(TextBlockPage { new: TextBlockPage::new });
xaml_class!(TextBlockPage, "/Pages/TextBlockPage.xaml");

impl TextBlockPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
