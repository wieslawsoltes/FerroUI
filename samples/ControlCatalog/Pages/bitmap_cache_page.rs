//! Port of `Pages/BitmapCachePage.xaml.cs`: the class of the document
//! `Pages/BitmapCachePage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct BitmapCachePage {
    base: ContentPage,
}

content_page_class!(BitmapCachePage);
ferro_class_info!(BitmapCachePage { new: BitmapCachePage::new });
xaml_class!(BitmapCachePage, "/Pages/BitmapCachePage.xaml");

impl BitmapCachePage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
