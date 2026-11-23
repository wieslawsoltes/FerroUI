//! Not from upstream: the page of WinUI's ListView
//! (docs/porting/winui-calculator-gaps.md, LV-1), the class of `Pages/ListViewPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct ListViewPage {
    base: ContentPage,
}

content_page_class!(ListViewPage);
ferro_class_info!(ListViewPage { new: ListViewPage::new });
xaml_class!(ListViewPage, "/Pages/ListViewPage.xaml");

impl ListViewPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
