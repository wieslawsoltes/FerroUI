//! Not from upstream: the page of the WinUI controls of `ferroui-controls`
//! (docs/porting/winui-controls.md), the class of `Pages/WinUIControlsPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct WinUIControlsPage {
    base: ContentPage,
}

content_page_class!(WinUIControlsPage);
ferro_class_info!(WinUIControlsPage { new: WinUIControlsPage::new });
xaml_class!(WinUIControlsPage, "/Pages/WinUIControlsPage.xaml");

impl WinUIControlsPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
