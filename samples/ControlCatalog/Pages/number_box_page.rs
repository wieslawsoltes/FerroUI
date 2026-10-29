//! Not from upstream: the page of WinUI's NumberBox
//! (docs/porting/winui-controls.md), the class of `Pages/NumberBoxPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct NumberBoxPage {
    base: ContentPage,
}

content_page_class!(NumberBoxPage);
ferro_class_info!(NumberBoxPage { new: NumberBoxPage::new });
xaml_class!(NumberBoxPage, "/Pages/NumberBoxPage.xaml");

impl NumberBoxPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
