//! Not from upstream: the page of WinUI's PagerControl
//! (docs/porting/winui-controls.md, G2), the class of `Pages/PagerControlPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct PagerControlPage {
    base: ContentPage,
}

content_page_class!(PagerControlPage);
ferro_class_info!(PagerControlPage { new: PagerControlPage::new });
xaml_class!(PagerControlPage, "/Pages/PagerControlPage.xaml");

impl PagerControlPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
