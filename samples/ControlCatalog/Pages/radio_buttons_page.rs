//! Not from upstream: the page of WinUI's RadioButtons
//! (docs/porting/winui-controls.md), the class of `Pages/RadioButtonsPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct RadioButtonsPage {
    base: ContentPage,
}

content_page_class!(RadioButtonsPage);
ferro_class_info!(RadioButtonsPage { new: RadioButtonsPage::new });
xaml_class!(RadioButtonsPage, "/Pages/RadioButtonsPage.xaml");

impl RadioButtonsPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
