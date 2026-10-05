//! Port of `Pages/SliderPage.xaml.cs`: the class of the document
//! `Pages/SliderPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct SliderPage {
    base: ContentPage,
}

content_page_class!(SliderPage);
ferro_class_info!(SliderPage { new: SliderPage::new });
xaml_class!(SliderPage, "/Pages/SliderPage.xaml");

impl SliderPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
