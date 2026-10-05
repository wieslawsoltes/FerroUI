//! Port of `Pages/LayoutTransformControlPage.xaml.cs`: the class of the document
//! `Pages/LayoutTransformControlPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct LayoutTransformControlPage {
    base: ContentPage,
}

content_page_class!(LayoutTransformControlPage);
ferro_class_info!(LayoutTransformControlPage { new: LayoutTransformControlPage::new });
xaml_class!(LayoutTransformControlPage, "/Pages/LayoutTransformControlPage.xaml");

impl LayoutTransformControlPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
