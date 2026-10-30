//! Not from upstream: the page of WinUI's InfoBar
//! (docs/porting/winui-controls.md), the class of `Pages/InfoBarPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct InfoBarPage {
    base: ContentPage,
}

content_page_class!(InfoBarPage);
ferro_class_info!(InfoBarPage { new: InfoBarPage::new });
xaml_class!(InfoBarPage, "/Pages/InfoBarPage.xaml");

impl InfoBarPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
