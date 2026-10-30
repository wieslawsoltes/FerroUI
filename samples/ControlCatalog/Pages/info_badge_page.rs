//! Not from upstream: the page of WinUI's InfoBadge
//! (docs/porting/winui-controls.md), the class of `Pages/InfoBadgePage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct InfoBadgePage {
    base: ContentPage,
}

content_page_class!(InfoBadgePage);
ferro_class_info!(InfoBadgePage { new: InfoBadgePage::new });
xaml_class!(InfoBadgePage, "/Pages/InfoBadgePage.xaml");

impl InfoBadgePage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
