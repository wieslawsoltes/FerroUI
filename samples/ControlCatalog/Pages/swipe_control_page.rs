//! Not from upstream: the page of WinUI's SwipeControl
//! (docs/porting/winui-calculator-gaps.md, SW-1), the class of `Pages/SwipeControlPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct SwipeControlPage {
    base: ContentPage,
}

content_page_class!(SwipeControlPage);
ferro_class_info!(SwipeControlPage { new: SwipeControlPage::new });
xaml_class!(SwipeControlPage, "/Pages/SwipeControlPage.xaml");

impl SwipeControlPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
