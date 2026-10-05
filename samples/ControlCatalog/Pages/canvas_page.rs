//! Port of `Pages/CanvasPage.xaml.cs`: the class of the document
//! `Pages/CanvasPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct CanvasPage {
    base: ContentPage,
}

content_page_class!(CanvasPage);
ferro_class_info!(CanvasPage { new: CanvasPage::new });
xaml_class!(CanvasPage, "/Pages/CanvasPage.xaml");

impl CanvasPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
