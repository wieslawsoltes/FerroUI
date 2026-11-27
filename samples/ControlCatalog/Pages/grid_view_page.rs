//! Not from upstream: the page of WinUI's GridView
//! (docs/porting/winui-calculator-gaps.md, LV-2), the class of `Pages/GridViewPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct GridViewPage {
    base: ContentPage,
}

content_page_class!(GridViewPage);
ferro_class_info!(GridViewPage { new: GridViewPage::new });
xaml_class!(GridViewPage, "/Pages/GridViewPage.xaml");

impl GridViewPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
