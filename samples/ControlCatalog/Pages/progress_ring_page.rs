//! Not from upstream: the page of WinUI's ProgressRing
//! (docs/porting/winui-controls.md), the class of `Pages/ProgressRingPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct ProgressRingPage {
    base: ContentPage,
}

content_page_class!(ProgressRingPage);
ferro_class_info!(ProgressRingPage { new: ProgressRingPage::new });
xaml_class!(ProgressRingPage, "/Pages/ProgressRingPage.xaml");

impl ProgressRingPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
